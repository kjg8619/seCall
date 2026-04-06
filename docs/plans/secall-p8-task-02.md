---
type: task
plan: secall-p8
task_number: 2
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 02: Release 바이너리 배포

## Changed files

- `Cargo.toml` — `[profile.release]` 섹션 추가
- `.github/workflows/release.yml` — 신규 파일, GitHub Actions release workflow
- `crates/secall/Cargo.toml` — 필요시 binary 메타데이터 추가

## Change description

### 1단계: Release 프로필 최적화

`Cargo.toml` (workspace root)에 추가:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
```

- `lto = "fat"` — 전체 링크 타임 최적화 (바이너리 크기 감소 + 성능)
- `codegen-units = 1` — 최적화 극대화 (빌드 시간 증가하지만 release에서는 OK)
- `strip = true` — 디버그 심볼 제거 (바이너리 크기 대폭 감소)

### 2단계: GitHub Actions release workflow

`.github/workflows/release.yml` 신규 작성:

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-13

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }} -p secall
      - run: |
          cd target/${{ matrix.target }}/release
          tar czf secall-${{ matrix.target }}.tar.gz secall
          mv secall-${{ matrix.target }}.tar.gz ${{ github.workspace }}/
      - uses: actions/upload-artifact@v4
        with:
          name: secall-${{ matrix.target }}
          path: secall-${{ matrix.target }}.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            secall-aarch64-apple-darwin/secall-aarch64-apple-darwin.tar.gz
            secall-x86_64-apple-darwin/secall-x86_64-apple-darwin.tar.gz
          generate_release_notes: true
```

### 3단계: 릴리스 프로세스 문서화

사용 방법:

```bash
# 태그 생성 → 자동 릴리스
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions가 macOS aarch64/x86_64 빌드 → Releases 페이지에 업로드

# 설치 (다른 Mac에서)
curl -L https://github.com/hang-in/seCall/releases/latest/download/secall-aarch64-apple-darwin.tar.gz | tar xz
sudo mv secall /usr/local/bin/
```

### 4단계: ONNX Runtime 동적 로딩 확인

현재 `ort` crate는 `load-dynamic` feature → ONNX Runtime을 런타임에 동적 로드. 바이너리에 정적 링크하지 않으므로:
- ONNX 모델 사용 시 별도 설치 필요 (`brew install onnxruntime` 또는 `secall model download`에서 안내)
- ONNX 없이도 BM25 검색은 동작 (벡터 검색만 비활성)

### 5단계: lindera ko-dic embed 크기 확인

lindera의 `embed-ko-dic` feature는 한국어 사전을 바이너리에 내장 → 바이너리 크기 증가 (예상 20-40MB). release 프로필의 `strip = true`와 `lto = "fat"`으로 최소화.

빌드 후 크기 확인:

```bash
cargo build --release -p secall
ls -lh target/release/secall
```

## Dependencies

- GitHub Actions — `dtolnay/rust-toolchain`, `softprops/action-gh-release`
- macOS runner — `macos-latest` (aarch64), `macos-13` (x86_64)
- 기존 `ci.yml` — 변경하지 않음, release.yml은 별도 트리거 (tag push만)

## Verification

```bash
# 1. Release 프로필로 빌드 확인
cargo build --release -p secall

# 2. 바이너리 크기 확인 (strip 적용)
ls -lh target/release/secall

# 3. 바이너리 실행 확인
./target/release/secall --version
./target/release/secall status

# 4. release workflow 문법 검증
# Manual: `gh workflow view release.yml` 또는 Actions 탭에서 확인
# 또는 act 도구로 로컬 테스트: act -j build --dryrun

# 5. 실제 릴리스 테스트 (태그 생성)
# Manual: git tag v0.2.0-rc1 && git push origin v0.2.0-rc1
# → GitHub Actions 빌드 성공 + Releases 페이지에 tar.gz 업로드 확인
```

## Risks

- **빌드 시간**: `lto = "fat"` + `codegen-units = 1`은 release 빌드 시간을 5-15분으로 증가시킴. CI에서만 사용하므로 로컬 개발에는 영향 없음
- **macOS 코드 사인**: GitHub Actions macOS runner는 ad-hoc 서명. Gatekeeper 경고 가능 → `xattr -cr secall`로 해제 안내 필요
- **lindera 크로스 빌드**: x86_64 빌드 시 macos-13 runner 사용 (네이티브 x86_64). 크로스 컴파일은 lindera embed 때문에 복잡할 수 있음
- **ONNX Runtime 누락**: 벡터 검색 없이도 BM25는 동작하지만, 사용자 혼란 방지를 위해 README에 안내 필요
- **ort crate 동적 로딩**: `load-dynamic` feature는 런타임에 `libonnxruntime.dylib`를 찾음. 없으면 벡터 검색 비활성 (graceful)

## Scope boundary

수정 금지 파일:
- `.github/workflows/ci.yml` — 기존 CI 변경 금지
- `crates/secall-core/` — 코어 라이브러리 변경 불필요
- `crates/secall/src/commands/` — 커맨드 로직 변경 불필요 (Task 01 범위)
- `README.md` — 설치 안내 업데이트는 릴리스 후 별도
