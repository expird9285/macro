# 제거 작업 검증 안내

대상: `origin/claude/amazing-meitner-4kmkxh` (이 안내를 쓸 때 리모트 HEAD는 `5a1952a3`)
목적: billing 무력화, canvas / Office·design 편집기 / CRM / mail / calendar / 코딩 에이전트 제거가 빠짐없이, 그리고 남은 기능을 깨뜨리지 않고 끝났는지 확인한다.

원칙
- 싼 검사부터 순서대로 돌린다. 앞 단계가 깨졌으면 뒤 단계 결과는 믿지 않는다.
- 실패는 "제거 때문에 생긴 것"과 "원래 실패하던 것"을 구분한다. 구분법은 6절에 있다.
- 못 돌린 검사는 pass로 적지 않고 명령과 막힌 이유를 적는다.
- 검증 중 발견한 것을 고칠 때는 작은 커밋으로 나눈다. `main`에는 커밋하지 않는다.

---

## 0. 시작 전 확인

```bash
git fetch origin
git status -sb                       # 작업 트리 clean 이어야 함
git log --oneline -5 origin/claude/amazing-meitner-4kmkxh
```

- **푸시 안 된 작업이 다른 기기에 남아 있으면 먼저 푸시한다.** 아래 1절의 웹/SDK 잔재는 리모트 `5a1952a3` 기준으로 확인한 것이라, 다른 기기에 미푸시 커밋이 있으면 사실과 다를 수 있다.
- 리모트 브랜치는 히스토리가 다시 쓰여 있다(force push). 이 브랜치를 예전에 pull해 둔 체크아웃이 있으면 `git pull`이 아니라 `git fetch` 후 `git reset --hard origin/claude/amazing-meitner-4kmkxh`로 맞춘다. 이때 로컬에만 있는 커밋이 없는지 먼저 확인한다.
- 검증용 DB는 개발 DB와 분리한다.

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/macrodb_verify
```

---

## 1. 이미 보이는 잔재 (정적 grep 결과, 리모트 `5a1952a3`)

아래는 파일 내용을 grep해서 본 것이다. 컴파일이나 실행으로 확인한 것은 아니다. "의도적으로 남김"인지 "빠뜨림"인지는 2절 허용 목록으로 판단한다.

| 영역 | 확인된 잔재 |
|---|---|
| `apps/web/src/features/` | `calendar`, `calendar-team`, `calendar-view`, `block-calendar`, `crm`, `block-email`, `email-compose`, `email-message`, `email-thread`, `email-view`, `reminders`, `scheduling` 디렉터리가 그대로 있다 |
| `apps/web/src/lib/service-clients/` | `service-calendar`, `service-email`, `service-stripe` 생성 클라이언트 |
| `apps/web` 설정 | `scripts/services.ts`, `scripts/generate-api-schema.ts`, `src/lib/core/constant/servers.ts`, `justfile`(`local-email`, `test-email-rendering*`)이 email/calendar 서비스를 가리킨다 |
| `packages/sdk` | calendar 53개, crm 39개, reminder 23개, email 17개 파일이 언급한다 |
| `.sqlx/` | 삭제 대상 테이블 이름이 들어간 캐시 파일이 약 980개(전체 약 2960개). 삭제된 쿼리의 캐시가 안 지워진 것으로 보인다 |
| 루트 `justfile` | `local-e2e-services`에 `email_service`가 남아 있고 `test-email-rendering*`도 있다 |
| 기타 | `.github/workspace-dep-closures.json`이 `email_service`를 언급한다. `.agents/rules/infra/pulumi.md`, `.agents/skills/audit/SKILL.md`, `.agents/skills/live-debug/SKILL.md`도 마찬가지다. `docs/`는 calendar 32개, crm 20개 파일이 언급한다 |

Rust 서비스 디렉터리(`services/email_service`, `calendar_service` 등)와 `infra/stacks`의 mail/calendar 스택, 해당 crate는 삭제돼 있었다. 워크스페이스 `members`가 가리키는 누락 디렉터리도 없었다.

웹/SDK 쪽을 일부러 미룬 것이라면 이 표는 "남은 일" 목록이다. 끝났다고 판단했다면 5절의 웹 검사에서 걸러져야 한다.

---

## 2. 의도적으로 남긴 것 (허용 목록)

아래에 해당하면 잔재가 아니다.

- 일반 enum variant: `EntityType::EmailThread/CalendarEvent/CrmCompany/Reminder`, `PropertyEntityType::*`, `FileType`. 저장된 데이터 호환용이다.
- Soup 필드 `is_email_attachment`. postcard 필드 순서 때문에 남겼고 값은 항상 `false`다.
- `xtask_local`의 `Port::Email`. 포트 오프셋이 테스트에 고정돼 있다.
- `contacts` crate와 `contacts_*` 테이블, `in_progress_email_link`, `mobile_welcome_email`.
- SES / 시스템 메일 발송 및 `email_suppression`, FusionAuth 로그인.
- billing: 삭제가 아니라 무력화다. AI admission은 항상 허용하고 `ai_usage` 계량은 유지한다 (`143d04af`).
- `graphql_soup`의 reminder stub (옛 클라이언트용).

---

## 3. 잔재 스캔

```bash
EXCL=(':!crates/macro_db_client/migrations' ':!.sqlx' ':!*.lock' ':!bun.lock' ':!apps/docs/changelog')
for k in 'calendar' 'crm' 'reminder' 'gmail|outlook|microsoft' \
         'email_service|email-service|calendar_service|calendar-service' \
         'coding[_-]agent|cursor_cloud|codex_cloud|claude_cloud' \
         'pptx|docx_engine|fig_engine|psd_engine'; do
  echo "== $k"
  git grep -liE "$k" -- . "${EXCL[@]}" | awk -F/ '{print $1"/"$2}' | sort | uniq -c | sort -rn | head -15
done
```

- 나온 디렉터리를 열어 2절 허용 목록에 해당하는지 본다. 해당하지 않으면 빠뜨린 것이다.
- `microsoft`, `canvas`, `calendar`는 일반 단어로도 쓰이니 문맥을 읽고 판단한다.

삭제된 테이블 이름을 코드가 아직 쓰는지(가장 중요):

```bash
M=crates/macro_db_client/migrations/20261025111500_drop_deprecated_tables.sql
T=$(grep -oE 'DROP TABLE IF EXISTS [a-z_]+' $M | awk '{print $5}' | paste -sd'|')
git grep -nwE "$T" -- crates services tooling ':!crates/macro_db_client/migrations' ':!.sqlx'
```

- 기대값: 주석이나 문자열 외에는 없음. `reminder` 같은 짧은 이름은 일반 단어와 겹친다.

죽은 문서 링크:

```bash
git ls-files '*.md' | while read -r f; do
  grep -oE '\]\(([^)#]+\.md)' "$f" | sed 's/](//' | while read -r l; do
    case "$l" in http*) continue;; esac
    [ -e "$(dirname "$f")/$l" ] || echo "$f -> $l"
  done
done
```

---

## 4. DB 마이그레이션 검증

마이그레이션 파일: `crates/macro_db_client/migrations/20261025111500_drop_deprecated_tables.sql`

### 4-1. 빈 DB에서 처음부터 적용

```bash
sqlx database drop -y; sqlx database create
sqlx migrate run --source crates/macro_db_client/migrations
```

### 4-2. 업그레이드 경로 (직전 버전 → 이 마이그레이션)

```bash
PREV=$(ls crates/macro_db_client/migrations | sort | tail -2 | head -1 | cut -d_ -f1)
sqlx database drop -y; sqlx database create
sqlx migrate run --source crates/macro_db_client/migrations --target-version "$PREV"

snap() { psql "$DATABASE_URL" -Atc "
  select 'view', schemaname||'.'||viewname from pg_views where schemaname='public'
  union all select 'matview', matviewname from pg_matviews
  union all select 'fk', conrelid::regclass::text||':'||conname from pg_constraint where contype='f'
  union all select 'trigger', tgrelid::regclass::text||':'||tgname from pg_trigger where not tgisinternal
  union all select 'func', p.proname from pg_proc p join pg_namespace n on n.oid=p.pronamespace where n.nspname='public'
  union all select 'index', indexname from pg_indexes where schemaname='public'
  order by 1,2"; }

snap > /tmp/before.txt
sqlx migrate run --source crates/macro_db_client/migrations
snap > /tmp/after.txt
diff /tmp/before.txt /tmp/after.txt | grep '^<'
```

사라진 줄을 하나씩 읽는다. `DROP TABLE ... CASCADE`는 의존하는 객체를 말없이 같이 지우기 때문이다.

- 삭제 대상 테이블에 속한 index, trigger, fk는 사라지는 게 정상이다.
- **남겨야 할 테이블이 가진 FK, view, function이 사라졌다면 문제다.** 예: `contacts_*`나 `User` 등을 가리키던 FK, soup 관련 view.
- 사라진 view/function을 남은 Rust 코드나 SQL이 쓰는지 `git grep`으로 확인한다.

### 4-3. 테이블 존재 여부

```bash
LIST=$(grep -oE 'DROP TABLE IF EXISTS [a-z_]+' $M | awk '{print $5}' | paste -sd,)

# 지워졌어야 하는 테이블: 결과가 비어야 한다
psql "$DATABASE_URL" -Atc "select table_name from information_schema.tables
  where table_schema='public' and table_name = any(string_to_array('$LIST', ','))"

# 접두어만 같은데 남은 테이블: 하나씩 의도된 것인지 판단한다
psql "$DATABASE_URL" -Atc "select table_name from information_schema.tables
  where table_schema='public'
    and table_name ~ '^(email_|calendar|crm_|reminder|harness|agent_preview|scheduling_)'"

# 유지해야 하는 테이블: 모두 보여야 한다
psql "$DATABASE_URL" -Atc "select table_name from information_schema.tables
  where table_schema='public' and (table_name like 'contacts\_%'
    or table_name in ('in_progress_email_link','mobile_welcome_email'))"
```

### 4-4. 배포 순서 주의 (기존 DB가 있는 경우)

이 저장소 규칙상 마이그레이션은 서비스 배포보다 먼저 적용된다. 이미 돌고 있는 환경에서 이 마이그레이션이 먼저 실행되면 아직 옛 바이너리가 지워진 테이블을 조회하다가 실패한다. 새 DB로 처음 띄우는 팀 서버라면 상관없다. 기존 데이터가 있다면 먼저 `pg_dump`한 사본에서 4-2를 한 번 돌려 본다. 운영 DB에는 직접 적용하지 않는다.

---

## 5. 빌드 / 정적 검사

위 DB(4-2에서 마이그레이션이 모두 적용된 상태)를 켜 둔 채 진행한다. 그래야 `sqlx` 매크로가 실제 스키마로 쿼리를 검사한다.

### 5-1. Rust

```bash
export SQLX_OFFLINE=false DATABASE_URL=postgres://user:password@localhost:5432/macrodb_verify
cargo fmt --all -- --check
cargo check --workspace --all-features --all-targets --exclude sync_service --keep-going
```

- 삭제된 테이블을 쓰는 쿼리가 남아 있으면 이 단계에서 컴파일 에러가 난다. 삭제된 crate를 가리키는 `Cargo.toml` 의존성이나 `use`도 여기서 잡힌다.
- 다음으로 `.sqlx` 캐시를 맞춘다.

```bash
# nix develop 안에서
cargo sqlx prepare --workspace --check -- --workspace --exclude sync_service --all-features
# 실패하면 재생성 후 커밋: just prepare_db
```

- 현재 `.sqlx`에 삭제된 쿼리의 캐시가 많이 남아 있으므로 재생성하면 삭제가 대량으로 생기는 게 정상이다. 새 쿼리 캐시가 생기거나 남은 기능의 캐시가 사라지면 점검한다.
- 캐시 기준 검사:

```bash
SQLX_OFFLINE=true just rust-check
just clippy
```

- 의존성 / 생성물 일치:

```bash
cargo x deps --check          # Cargo.toml이 바뀌었으면 필수. 실패하면 just hakari 후 커밋
cargo x workflows --check
cargo x kafka-topics --check
```

### 5-2. Rust 테스트

```bash
unset SQLX_OFFLINE
export DATABASE_URL=postgres://user:password@localhost:5432/macrodb_verify
just run_dbs -d && just setup_test_envs     # Redis 등 필요한 인프라
CARGO_BUILD_JOBS=2 cargo test --workspace --exclude sync_service --no-fail-fast 2>&1 | tee /tmp/test.log
grep -E '^error|FAILED|failed' /tmp/test.log | head -50
```

- 한 번에 안 되면 영향 큰 패키지부터 `cargo test -p <패키지>`로 나눠 돌린다: `macro_db_client`, `documents`, `properties`, `entity_access`, `channels`, `call`, `soup`, `notification*`, `authentication_service`, `document_storage_service`, `document_cognition_service`, `mcp_service`, `ai_billing*`.
- billing은 "무력화"가 맞는지 테스트나 코드로 확인한다. AI 호출이 항상 허용되는지, `ai_usage` 기록은 계속 남는지 본다.
- DB 테스트가 `max_locks_per_transaction` 때문에 실패하면 `docs/development/database.md`의 "Pre-migrated test template" 절을 따른다.

### 5-3. 웹

```bash
cd apps/web
bun install --frozen-lockfile
bun run check          # graphql 캐시 스키마, dynamic-ui 스키마, tsc, biome
bun run lint
bun run knip           # 삭제 후 아무도 안 쓰는 파일/export 탐지. 제거 작업에서 특히 유용하다
bun run test
bun run build
```

- `bun run check` 실패 중 "모듈을 찾을 수 없음"은 삭제된 화면을 아직 import하는 곳이다.
- 1절 표대로 `features/calendar` 등이 그대로면 knip에서 연결이 끊긴 파일이 대량으로 나온다. 의도적으로 미룬 것인지 판단한다.
- 생성물(`gen-api`, `gen-graphql`, `gen-tools`)을 다시 만든 뒤 diff가 비어야 한다. 원격 CI의 `check_generated.yml`이 실제로 어떤 명령을 도는지 열어서 같은 명령을 로컬에서 돌리는 게 가장 정확하다.
- SDK: `packages/sdk/package.json`의 스크립트로 빌드와 테스트를 돌리고, `emailAttachmentId` 같은 삭제된 필드가 openapi와 SDK에 안 남았는지 `git grep`한다.

### 5-4. 인프라

```bash
bun run --cwd infra check        # 배포(pulumi up)는 절대 실행하지 않는다
git grep -nE 'email|calendar|crm|stripe' -- infra ':!infra/stacks/email-suppression' | head -30
```

- `email-suppression` 스택은 SES용이라 유지가 맞다. 다른 hit는 설정 키, Doppler 키, 서비스 URL 표(`infra/packages/shared/src/service_urls.ts`)를 본다.

### 5-5. 서비스 URL 사본 4곳

`.agents/rules/config-and-service-urls.md` 기준으로 아래 네 곳의 목록이 서로 같아야 한다.

- `crates/macro_service_urls`
- `infra/packages/shared/src/service_urls.ts`
- `apps/web/src/lib/core/constant/servers.ts`
- `packages/sdk/src/config.ts`

---

## 6. 실패했을 때 가르는 법

1. 에러가 삭제된 것(crate, 모듈, 테이블, 컴포넌트)을 가리키는가? → 제거 때문에 생긴 문제다. 호출부를 정리한다.
2. 에러가 남은 기능인데 삭제와 상관없어 보이는가? → `origin/main`에서 같은 명령이 실패하는지 본다.

```bash
git worktree add /tmp/main-wt origin/main     # 디스크가 넉넉할 때만
cd /tmp/main-wt && cargo test -p <패키지>
```

3. main에서도 실패하면 기존 문제다. 보고서에 따로 적고 이 작업의 책임으로 묶지 않는다.
4. 테스트를 건너뛰거나 지워서 통과시키지 않는다. 삭제된 기능의 테스트면 지우는 게 맞지만, 남은 기능의 테스트는 원인을 고친다.

---

## 7. 동작 확인 (가능하면)

1. 로컬 스택을 띄운다. 방법은 `docs/development/running-locally.md`의 "Run the local stack"을 따른다.
   - 루트 `justfile`의 `local-e2e-services`에 `email_service`가 남아 있으면(1절) 스택 기동이 실패하니 먼저 고친다.
   - 서비스가 시작 시 삭제된 설정 키나 큐를 요구하며 죽는지 로그를 본다. `MacroConfig`는 누락 값에서 시작 실패하도록 돼 있다.
2. 브라우저(또는 `browser-verifier` 서브에이전트, `docs/agent-guide/README.md` 기준)로 확인한다.
   - 로그인(FusionAuth), 문서 생성/편집, 채널 메시지, 검색, 설정.
   - 사이드바와 명령 팔레트, 설정에 mail / calendar / CRM / canvas / 코딩 에이전트 / 결제 메뉴가 없다.
   - 삭제된 경로(`/email`, `/calendar`, `/crm`, `/canvas` 등)로 직접 들어가도 흰 화면 없이 404나 리다이렉트로 처리된다.
   - 콘솔과 네트워크 탭에 삭제된 서비스로 가는 요청과 4xx/5xx가 없다.
3. AI 채팅: 요청이 사용량/결제 때문에 막히지 않고, 도구 목록에 mail / calendar / CRM / 코딩 도구가 없다.
4. 서비스 로그에서 삭제된 테이블 이름이 들어간 SQL 에러(`relation "..." does not exist`)가 없는지 `grep`한다.

---

## 8. 마무리

- 이 저장소 규칙대로 `audit` 스킬을 `auditor` 서브에이전트로 돌린다 (`.agents/skills/audit/SKILL.md`). 변경 파일에서 필요한 검사를 자동으로 모아 pass/fail/skipped 표를 준다.
- `just check full`을 한 번 돌린다 (포맷, 린트, ast-grep 규칙, tsc, clippy).
- 바뀐 디렉터리의 `AGENTS.md`(`CLAUDE.md`는 심볼릭 링크)가 삭제된 기능을 아직 설명하는지 확인한다. `properties`, `call`, `channels`, `macro_db_client`, `model`, `models_properties` 등.
- 규칙 문서 정리: CS-72(KMS 시크릿 저장 규칙)가 삭제된 Microsoft/Gmail 연동 예시를 쓰는지, `.agents/rules/**`와 `docs/README.md` 색인이 삭제된 문서를 가리키는지 본다.
- Doppler: 삭제된 서비스의 `dev`/`prd` 키 정리는 코드 밖 작업이다. 지우기 전에 목록만 만들어 사용자에게 확인받는다.

## 9. 결과 보고 양식

| 결과 | 검사 | 명령 | 근거 |
|---|---|---|---|
| pass / fail / skipped | 예: Rust 컴파일 | `cargo check --workspace ...` | 에러 수, 로그 위치, 막힌 이유 |

실패는 위에 둔다. 각 실패에는 "제거 때문" / "main에서도 실패" / "원인 미상" 중 하나를 붙인다.
