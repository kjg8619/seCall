---
type: task
plan: secall-p11
task_number: 2
title: ORT 진짜 batch inference
status: draft
depends_on: []
parallel_group: A
updated_at: 2026-04-07
---

# Task 02 — ORT 진짜 batch inference

## Changed files

| 파일 | 변경 유형 |
|------|----------|
| `crates/secall-core/src/search/embedding.rs:147-212` | 수정 — `run_inference()` → `run_inference_batch()` batch axis 처리 |
| `crates/secall-core/src/search/embedding.rs:231-247` | 수정 — `embed_batch()` 내부 for loop 제거, 진짜 batch 호출 |
| `crates/secall-core/src/search/embedding.rs:348+` | 수정 — 관련 테스트 추가 |

## Change description

### 현재 문제

`OrtEmbedder::embed_batch()` (line 231-247):
```rust
// 현재: 가짜 batch — 텍스트별 개별 inference
for text in &texts {
    results.push(Self::run_inference(&mut session, &tokenizer, text)?);
}
```

`run_inference()` (line 147-212): 입력이 `(1, seq_len)` 고정 — batch axis 없음.

### 1. `run_inference_batch()` 구현

기존 `run_inference()`를 보존하고, 새 `run_inference_batch()` 추가:

```rust
fn run_inference_batch(
    session: &mut ort::session::Session,
    tokenizer: &tokenizers::Tokenizer,
    texts: &[String],
) -> Result<Vec<Vec<f32>>>
```

**tokenizer batch**:
```rust
let encodings = tokenizer
    .encode_batch(texts.iter().map(|t| t.as_str()).collect::<Vec<_>>(), true)
    .map_err(|e| anyhow!("batch tokenize failed: {e}"))?;
```

**padding**: batch 내 최대 seq_len에 맞춰 padding + attention_mask 구성:
```rust
let max_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0);
let batch_size = texts.len();

let mut input_ids = Array2::<i64>::zeros((batch_size, max_len));
let mut attention_mask = Array2::<i64>::zeros((batch_size, max_len));

for (i, enc) in encodings.iter().enumerate() {
    let ids = enc.get_ids();
    let mask = enc.get_attention_mask();
    for (j, (&id, &m)) in ids.iter().zip(mask.iter()).enumerate() {
        input_ids[[i, j]] = id as i64;
        attention_mask[[i, j]] = m as i64;
    }
}
```

**single session.run()**: shape `(batch_size, max_len)` → output `(batch_size, max_len, dim)`:
```rust
let outputs = session.run(ort::inputs![
    "input_ids" => TensorRef::from_array_view(input_ids.view())?,
    "attention_mask" => TensorRef::from_array_view(attention_mask.view())?,
])?;

let hidden = outputs["last_hidden_state"].try_extract_array::<f32>()?;
// shape: [batch_size, max_len, dim]
```

**mean pooling per sample**: attention_mask 기반:
```rust
let dim = hidden.shape()[2];
let mut results = Vec::with_capacity(batch_size);
for i in 0..batch_size {
    let mask_sum = attention_mask.row(i).iter().map(|&m| m as f32).sum::<f32>().max(1e-9);
    let mut embedding = vec![0.0f32; dim];
    for j in 0..max_len {
        let m = attention_mask[[i, j]] as f32;
        for d in 0..dim {
            embedding[d] += hidden[[i, j, d]] * m;
        }
    }
    for e in embedding.iter_mut() { *e /= mask_sum; }
    // L2 normalize
    let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 { for e in embedding.iter_mut() { *e /= norm; } }
    results.push(embedding);
}
```

### 2. embed_batch() 수정

```rust
async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let session = Arc::clone(&self.session);
    let tokenizer = Arc::clone(&self.tokenizer);
    let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let mut session = session.lock()
            .map_err(|_| anyhow!("ort session lock poisoned"))?;
        Self::run_inference_batch(&mut session, &tokenizer, &texts)
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking join error: {e}"))?
}
```

Mutex lock은 여전히 batch 당 1회 — batch 내 모든 텍스트가 한 번의 lock + 한 번의 session.run()으로 처리됨.

### 3. embed() 단일 텍스트 (기존 유지)

`embed()` 는 기존 `run_inference()` 사용 유지 (단일 텍스트에 batch overhead 불필요).

### 4. 메모리 관리

batch_size 32 기준 최대 메모리:
- `input_ids`: 32 × 512 × 8 bytes = ~128KB
- `attention_mask`: ~128KB
- `last_hidden_state`: 32 × 512 × 1024 × 4 = ~64MB

M1 Max 64GB에서 충분. batch_size가 너무 크면 OOM 가능하므로 Task 04에서 CLI로 조절 가능하게 함.

### 5. tokenizer padding 설정

`tokenizers::Tokenizer`의 padding 설정이 필요할 수 있음:
```rust
use tokenizers::PaddingParams;
tokenizer.with_padding(Some(PaddingParams {
    strategy: tokenizers::PaddingStrategy::BatchLongest,
    ..Default::default()
}));
```

또는 수동 padding (위 구현처럼 `Array2::zeros`로 직접 패딩) — 더 명시적이고 안전.

## Dependencies

- 없음 (Task 01과 parallel_group A로 병렬 가능)
- `tokenizers` crate의 `encode_batch()` 사용 — 이미 의존성에 포함

## Verification

```bash
# 타입 체크
cargo check --all

# 전체 테스트
cargo test --all

# clippy
cargo clippy --all-targets -- -D warnings

# ORT 실제 batch 테스트 (모델 필요, --ignored)
cargo test -p secall-core test_ort_embed -- --ignored

# 기존 embedder trait 테스트
cargo test -p secall-core test_embedder_trait
```

## Risks

- **ONNX 모델 batch 호환성**: BGE-M3 ONNX 모델이 dynamic batch axis를 지원하는지 확인 필요. 대부분의 HuggingFace export 모델은 지원하지만, 고정 batch=1로 export된 경우 런타임 에러 발생. 대응: `run_inference_batch()` 실패 시 기존 sequential fallback.
- **메모리 사용량 증가**: batch_size × max_seq_len × dim 텐서가 한 번에 할당됨. batch_size 32이면 ~64MB로 문제 없으나, 비정상적으로 긴 텍스트(seq_len > 8192)가 있으면 주의.
- **기존 embed() 동작 변경 없음**: 단일 텍스트 경로는 기존 `run_inference()` 유지.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/vector.rs` — Task 01, 03 영역
- `crates/secall-core/src/store/db.rs` — Task 01 영역
- `crates/secall/src/commands/embed.rs` — Task 03, 04 영역
- `crates/secall/src/main.rs` — Task 03, 04 영역
- Ollama/OpenAI embedder 코드 (`embedding.rs` 내 해당 구현) — 변경 금지
