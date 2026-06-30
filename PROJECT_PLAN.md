# 1. Tese central da arquitetura

A lib não deve tentar “entender PHP”, “entender Node”, “entender Django” ou “entender Laravel” profundamente logo de cara. Isso vira areia movediça.

Ela deve entender **um protocolo neutro de observação**.

Cada linguagem/framework fornece um adapter que transforma seus testes, requests, comandos ou funções em um formato canônico:

```txt
runtime/framework específico
        ↓
adapter
        ↓
Rewrit Protocol
        ↓
engine Rust
        ↓
normalização
        ↓
comparação
        ↓
relatório
        ↓
exit code para CI/agente
```

Essa é a decisão arquitetural mais importante.

A arquitetura deve ser **hexagonal**, com o core isolado e adapters nas bordas. O core não conhece Laravel, Pest, Vitest, Django, Encore ou Pytest. Ele conhece apenas: caso, contrato, observação, runtime, policy, divergência e relatório.

Cargo workspaces são uma boa base para esse projeto porque permitem manter múltiplos crates relacionados sob um mesmo workspace, compartilhando `Cargo.lock` e diretório de build. Isso encaixa bem numa lib com core, CLI, adapters e crates auxiliares. ([Documentação do Rust][1])

---

# 2. O produto em uma frase

**Rewrit é um parity engine open-source em Rust para validar que uma implementação candidata preserva contratos, tipos, efeitos colaterais, erros e comportamento observável de uma implementação de referência durante reescritas entre stacks.**

Ele não promete equivalência metafísica do software inteiro. Ele promete paridade **dentro dos contratos observados, declarados e versionados**. Esse detalhe é importante, porque “100% de paridade” precisa ser medido por cobertura de contratos, não por fé tribal.

---

# 3. Conceitos fundamentais

## 3.1 Reference e Candidate

Nunca use `legacy` e `new` no modelo interno. Use:

```txt
reference: implementação fonte da verdade
candidate: implementação sendo validada
```

Exemplo:

```txt
reference = Laravel/PHP
candidate = Encore/TypeScript

reference = Django/Python
candidate = Rust/Axum
```

Isso mantém a API limpa para refatoração, migração, rewrite, dual-run ou comparação entre duas versões modernas.

---

## 3.2 Case

Um **case** é uma unidade verificável de comportamento.

Pode nascer de:

```txt
teste Pest
teste PHPUnit
teste Vitest
teste Jest
teste Pytest
teste cargo test
request HTTP
comando CLI
job de fila
função instrumentada
contrato manual
```

Modelo mental:

```rust
pub struct Case {
    pub id: CaseId,
    pub suite_id: SuiteId,
    pub title: String,
    pub source_location: Option&amp;lt;SourceLocation&amp;gt;,
    pub tags: Vec&amp;lt;String&amp;gt;,
    pub contract_ref: Option&amp;lt;ContractRef&amp;gt;,
    pub required: bool,
}
```

O `id` é a alma do sistema. Ele conecta as pontas.

Exemplo:

```txt
billing.invoice.create.success
auth.login.invalid_password
orders.refund.partial
users.profile.update_email_conflict
```

---

## 3.3 Contract

Um **contract** descreve o que precisa ser equivalente.

Ele pode incluir:

```txt
entrada
saída
tipo
erro esperado
HTTP status
headers relevantes
mutação em banco
eventos emitidos
mensagens de fila
arquivos criados
logs importantes
política de tolerância
normalizadores aplicáveis
```

Formato canônico sugerido: JSON ou YAML com JSON Schema.

Serde é uma base natural para serialização/deserialização em Rust, e Schemars pode gerar JSON Schema a partir de tipos Rust com derive, o que ajuda a manter os schemas oficiais sincronizados com o modelo interno. ([Serde][2])

Exemplo de contrato:

```json
{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {
    "method": "POST",
    "path": "/api/invoices",
    "json": {
      "customer_id": "cus_123",
      "amount": "199.90",
      "currency": "BRL"
    }
  },
  "expect": {
    "status": 201,
    "json_schema": {
      "type": "object",
      "required": ["id", "amount", "currency", "status"],
      "properties": {
        "id": { "type": "string" },
        "amount": { "type": "string", "pattern": "^\\d+\\.\\d{2}$" },
        "currency": { "const": "BRL" },
        "status": { "const": "open" }
      }
    },
    "effects": [
      {
        "kind": "db.insert",
        "table": "invoices",
        "fields": ["id", "customer_id", "amount", "currency", "status"]
      }
    ]
  },
  "policy": "http_api_strict"
}
```

---

## 3.4 Observation

Uma **observation** é o que um runtime produziu ao executar um case.

```rust
pub struct Observation {
    pub case_id: CaseId,
    pub runtime_id: RuntimeId,
    pub status: CaseStatus,
    pub value: Option&amp;lt;CanonicalValue&amp;gt;,
    pub error: Option&amp;lt;CanonicalError&amp;gt;,
    pub stdout: CapturedText,
    pub stderr: CapturedText,
    pub exit_code: Option&amp;lt;i32&amp;gt;,
    pub duration_ms: u64,
    pub effects: Vec&amp;lt;Effect&amp;gt;,
    pub artifacts: Vec&amp;lt;Artifact&amp;gt;,
    pub metadata: BTreeMap&amp;lt;String, String&amp;gt;,
}
```

A observation não deve ser apenas “passou” ou “falhou”. Ela deve ser um envelope rico o suficiente para comparar comportamento.

---

## 3.5 Divergence

Uma **divergence** é uma diferença classificada.

Categorias principais:

```txt
missing_candidate_case
missing_reference_case
output_mismatch
type_mismatch
schema_mismatch
error_mismatch
side_effect_mismatch
stdout_mismatch
stderr_mismatch
exit_code_mismatch
timeout
flaky
adapter_error
infra_error
policy_allowed
waiver_expired
```

Exemplo de divergência:

```json
{
  "kind": "type_mismatch",
  "severity": "blocking",
  "case_id": "billing.invoice.create.success",
  "path": "$.amount",
  "reference": {
    "type": "string",
    "value": "199.90"
  },
  "candidate": {
    "type": "number",
    "value": 199.9
  },
  "message": "O runtime candidato retornou number, mas o contrato exige decimal como string."
}
```

Isso é perfeito para humanos e para agentes de código.

---

# 4. O fluxo funcional

```txt
1. load manifest
2. validate config
3. discover cases
4. resolve bindings
5. run reference
6. run candidate
7. collect observations
8. normalize
9. validate schemas
10. compare
11. classify divergences
12. apply policies and waivers
13. write reports
14. return exit code
```

Estado interno de um case:

```txt
discovered
  ↓
bound
  ↓
scheduled
  ↓
reference_running
  ↓
candidate_running
  ↓
observed
  ↓
normalized
  ↓
compared
  ↓
classified
  ↓
reported
```

Essa state machine evita aquela geleia operacional onde timeout, teste faltando, runtime quebrado e diferença semântica viram tudo “fail”.

---

# 5. Modos de operação

## 5.1 Mirror mode

Executa reference e candidate na mesma rodada.

```bash
rewrit run --mode mirror
```

Uso:

```txt
Laravel vs Encore
Django vs Rust
PHP service vs Node service
```

Vantagem: reduz risco de baseline velho.

---

## 5.2 Baseline mode

Captura uma referência congelada e depois compara o candidate contra ela.

```bash
rewrit capture --runtime reference
rewrit verify --runtime candidate
```

Uso:

```txt
migração longa
CI rápido
agentes trabalhando em ciclos pequenos
```

---

## 5.3 Contract mode

Não depende de testes existentes. Usa contratos canônicos escritos manualmente ou gerados.

```bash
rewrit verify --contracts contracts/**/*.json
```

Uso:

```txt
HTTP APIs
serviços internos
jobs
funções críticas
domínios com contratos estáveis
```

---

## 5.4 Audit mode

Verifica se todos os testes/contratos da referência existem no candidato.

```bash
rewrit audit
```

Uso:

```txt
garantir que todo teste PHP tenha equivalente Node
garantir que todo teste Django tenha equivalente Rust
impedir buracos silenciosos na migração
```

---

# 6. Estrutura do repositório

A estrutura deve começar organizada para crescer sem virar uma gaveta de cabos.

```txt
rewrit/
  Cargo.toml
  README.md
  LICENSE-MIT
  LICENSE-APACHE
  CHANGELOG.md
  CONTRIBUTING.md
  SECURITY.md
  CODE_OF_CONDUCT.md

  crates/
    rewrit-model/
      Cargo.toml
      src/
        lib.rs
        ids.rs
        case.rs
        contract.rs
        observation.rs
        value.rs
        error.rs
        effect.rs
        divergence.rs
        report.rs

    rewrit-core/
      Cargo.toml
      src/
        lib.rs
        compare/
          mod.rs
          comparator.rs
          diff.rs
          json.rs
          schema.rs
          error.rs
          effects.rs
        normalize/
          mod.rs
          pipeline.rs
          path.rs
          time.rs
          regex.rs
          ordering.rs
          http.rs
          php.rs
        policy/
          mod.rs
          engine.rs
          waiver.rs
          severity.rs
        validate/
          mod.rs
          schema.rs
          manifest.rs

    rewrit-engine/
      Cargo.toml
      src/
        lib.rs
        engine.rs
        planner.rs
        scheduler.rs
        runner/
          mod.rs
          process.rs
          timeout.rs
          env.rs
          sandbox.rs
        discovery/
          mod.rs
          manifest.rs
          binding.rs
        store/
          mod.rs
          filesystem.rs
          baseline.rs
          cache.rs
        events.rs

    rewrit-protocol/
      Cargo.toml
      src/
        lib.rs
        ndjson.rs
        adapter.rs
        events.rs
        version.rs

    rewrit-report/
      Cargo.toml
      src/
        lib.rs
        terminal.rs
        json.rs
        ndjson.rs
        junit.rs
        sarif.rs
        html.rs

    rewrit-cli/
      Cargo.toml
      src/
        main.rs
        app.rs
        commands/
          init.rs
          doctor.rs
          discover.rs
          capture.rs
          verify.rs
          run.rs
          audit.rs
          explain.rs
          schema.rs
          report.rs

    rewrit-adapter-command/
      Cargo.toml
      src/
        lib.rs

    rewrit-adapter-http/
      Cargo.toml
      src/
        lib.rs
        server.rs
        request.rs
        response.rs

    rewrit-adapter-php/
      Cargo.toml
      src/
        lib.rs
        pest.rs
        phpunit.rs
        laravel.rs

    rewrit-adapter-node/
      Cargo.toml
      src/
        lib.rs
        vitest.rs
        jest.rs
        encore.rs

    rewrit-adapter-python/
      Cargo.toml
      src/
        lib.rs
        pytest.rs
        django.rs

    rewrit-adapter-rust/
      Cargo.toml
      src/
        lib.rs
        cargo_test.rs

  sdks/
    php/
      composer.json
      src/
        Rewrit.php
        PestPlugin.php
        PHPUnitExtension.php

    node/
      package.json
      src/
        index.ts
        vitest-reporter.ts
        jest-reporter.ts
        encore.ts

    python/
      pyproject.toml
      rewrit_pytest/
        __init__.py
        plugin.py

    rust/
      Cargo.toml
      src/
        lib.rs
        macros.rs

  docs/
    concepts/
      parity.md
      contracts.md
      observations.md
      policies.md
      side-effects.md
    protocol/
      adapter-protocol-v1.md
      observation-schema-v1.md
      report-schema-v1.md
    adapters/
      php-pest.md
      php-phpunit.md
      node-vitest.md
      node-encore.md
      python-pytest.md
      django.md
      rust-cargo-test.md
    migrations/
      laravel-to-encore.md
      django-to-rust.md
      laravel-to-node.md
    adr/
      0001-ndjson-adapter-protocol.md
      0002-reference-candidate-model.md
      0003-json-schema-contracts.md
      0004-no-language-parser-in-core.md

  examples/
    command-to-command/
    http-to-http/
    laravel-to-encore/
    django-to-rust/
    php-to-node-monolith/

  tests/
    fixtures/
      fake-adapters/
      reports/
      manifests/
    e2e/
      command_adapter.rs
      http_adapter.rs
      missing_case.rs
      side_effects.rs

  .github/
    workflows/
      ci.yml
      release.yml
    ISSUE_TEMPLATE/
    PULL_REQUEST_TEMPLATE.md
```

A estrutura respeita convenções comuns do Cargo para `src`, `tests`, `examples` e binários, deixando o projeto familiar para qualquer dev Rust que cair nele de paraquedas. ([Documentação do Rust][3])

---

# 7. Crates principais

## 7.1 `rewrit-model`

Responsável apenas pelo modelo canônico.

Não executa nada. Não chama processo. Não lê config. Não compara.

Contém:

```txt
Case
Contract
Observation
CanonicalValue
CanonicalError
Effect
Divergence
Report
Ids
SourceLocation
```

Esse crate deve ser extremamente estável.

Boas práticas:

```rust
#![forbid(unsafe_code)]

pub mod case;
pub mod contract;
pub mod divergence;
pub mod effect;
pub mod error;
pub mod ids;
pub mod observation;
pub mod report;
pub mod value;
```

Use `#[non_exhaustive]` em enums públicas para permitir evolução sem quebrar usuários:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DivergenceKind {
    MissingCandidateCase,
    OutputMismatch,
    TypeMismatch,
    SchemaMismatch,
    ErrorMismatch,
    SideEffectMismatch,
    Timeout,
    Flaky,
    InfraError,
}
```

---

## 7.2 `rewrit-core`

Responsável por lógica pura:

```txt
normalização
comparação
policy engine
schema validation
classificação
waivers
diff
```

Não deve saber como rodar PHP, Node ou Python.

Exemplo de trait:

```rust
pub trait Normalizer: Send + Sync {
    fn name(&amp;amp;self) -&amp;gt; &amp;amp;'static str;

    fn normalize(
        &amp;amp;self,
        observation: Observation,
        ctx: &amp;amp;NormalizeContext,
    ) -&amp;gt; Result&amp;lt;Observation, NormalizeError&amp;gt;;
}
```

Comparator:

```rust
pub trait Comparator: Send + Sync {
    fn name(&amp;amp;self) -&amp;gt; &amp;amp;'static str;

    fn compare(
        &amp;amp;self,
        reference: &amp;amp;Observation,
        candidate: &amp;amp;Observation,
        ctx: &amp;amp;CompareContext,
    ) -&amp;gt; Comparison;
}
```

Policy engine:

```rust
pub struct PolicyEngine {
    normalizers: Vec&amp;lt;Box&amp;lt;dyn Normalizer&amp;gt;&amp;gt;,
    comparators: Vec&amp;lt;Box&amp;lt;dyn Comparator&amp;gt;&amp;gt;,
    waivers: WaiverSet,
}
```

---

## 7.3 `rewrit-engine`

Responsável por orquestração:

```txt
carregar manifest
descobrir cases
resolver bindings
montar plano de execução
executar runtimes
limitar paralelismo
aplicar timeout
coletar events
chamar core
emitir events internos
```

Ele é o maestro. Não toca violino, não toca trombone, só impede a orquestra de virar um churrasco.

---

## 7.4 `rewrit-protocol`

Define o protocolo entre engine e adapters.

A decisão recomendada: **NDJSON via stdout ou arquivo**, com mensagens versionadas.

Por quê?

```txt
streaming
simples para qualquer linguagem
bom para monolitos gigantes
não exige servidor
não exige FFI
não exige plugin nativo Rust
fácil de debugar
```

TAP nasceu justamente como um protocolo simples entre producers de teste e harnesses, separando produção de resultados da apresentação. A ideia aqui é parecida, mas usando JSON estruturado para carregar diffs, efeitos, erros e schemas. ([Test Anything Protocol][4])

Evento exemplo:

```json
{"schema_version":"rewrit.event.v1","kind":"case_started","case_id":"billing.invoice.create.success","runtime_id":"reference"}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open","amount":"199.90"}}}
{"schema_version":"rewrit.event.v1","kind":"case_finished","case_id":"billing.invoice.create.success","runtime_id":"reference","duration_ms":82}
```

---

## 7.5 `rewrit-report`

Formatos de saída:

```txt
terminal
json
ndjson
junit
sarif
html
markdown
```

JUnit XML é útil porque várias ferramentas de CI entendem relatórios nesse estilo, apesar de o ecossistema ter variações de convenção. SARIF é um padrão OASIS para resultados de análise estática e pode ser útil para anotar problemas em PRs e painéis de qualidade. ([GitHub][5])

---

## 7.6 `rewrit-cli`

Binário principal.

Use `clap` para CLI. A documentação do crate cobre derive, builder API, cookbook e conceitos de CLI, então ele é uma escolha segura para uma ferramenta com muitos subcomandos. ([Docs.rs][6])

Comandos:

```bash
rewrit init
rewrit doctor
rewrit discover
rewrit capture
rewrit verify
rewrit run
rewrit audit
rewrit explain
rewrit schema
rewrit report
```

---

# 8. Manifesto do projeto

Arquivo raiz:

```txt
rewrit.toml
```

Exemplo para Laravel para Encore:

```toml
[project]
name = "billing-migration"
reference = "legacy_laravel"
candidate = "encore_ts"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.legacy_laravel]
adapter = "php:pest"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]
timeout_ms = 30000

[runtimes.legacy_laravel.env]
APP_ENV = "testing"
CACHE_DRIVER = "array"
QUEUE_CONNECTION = "sync"

[runtimes.encore_ts]
adapter = "node:vitest"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[runtimes.encore_ts.env]
NODE_ENV = "test"

[[suites]]
id = "billing"
title = "Billing domain"
source_glob = "tests/Feature/Billing/**/*.php"
policy = "http_api_strict"
required = true

[[bindings]]
case = "billing.invoice.create.success"
reference = "billing.invoice.create.success"
candidate = "billing.invoice.create.success"

[policies.http_api_strict]
compare_status = true
compare_json = true
compare_headers = true
compare_effects = true
numeric_epsilon = "0.000001"

[policies.http_api_strict.headers]
ignore = ["date", "x-request-id", "server"]

[policies.http_api_strict.json]
unordered_paths = ["$.items[*].metadata"]
ignore_paths = ["$.generated_at", "$.trace_id"]

[[normalizers]]
kind = "path"
replace_project_root = "&amp;lt;PROJECT_ROOT&amp;gt;"

[[normalizers]]
kind = "regex"
pattern = "\\b[0-9a-f]{32}\\b"
replacement = "&amp;lt;HEX32&amp;gt;"

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"

[[reports]]
kind = "junit"
path = ".rewrit/reports/junit.xml"

[[reports]]
kind = "sarif"
path = ".rewrit/reports/rewrit.sarif"
```

---

# 9. Storage local

```txt
.rewrit/
  baselines/
    legacy_laravel/
      current.jsonl
      2026-06-30T10-20-00Z.jsonl
  cache/
    discovery/
    schemas/
    normalized/
  reports/
    latest.json
    latest.ndjson
    junit.xml
    rewrit.sarif
    html/
  locks/
  tmp/
```

Contratos versionados devem ficar fora de `.rewrit`, dentro do repositório:

```txt
contracts/
  billing/
    invoice.create.success.json
    invoice.create.validation_error.json
  auth/
    login.success.json
    login.invalid_password.json
```

Regra prática:

```txt
contracts são código
baselines são evidência
reports são artefato
cache é descartável
```

---

# 10. Modelo canônico de valores

A maior fonte de dor será tipo. PHP, JS, Python e Rust discordam em detalhes venenosos.

Exemplo de diferenças:

```txt
PHP array pode ser lista ou mapa
JS tem undefined
Python tem None
Rust tem Option
JSON não tem Date
float não serve para dinheiro
headers HTTP são case-insensitive
ordem de objeto JSON não deveria importar
ordem de array normalmente importa
NaN e Infinity não existem em JSON padrão
```

Por isso, o modelo canônico não deve ser apenas `serde_json::Value`.

Sugestão:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CanonicalValue {
    Null,

    Absent,

    Bool {
        value: bool,
    },

    Integer {
        value: String,
    },

    Decimal {
        value: String,
    },

    Float {
        value: String,
    },

    String {
        value: String,
    },

    Bytes {
        base64: String,
        media_type: Option&amp;lt;String&amp;gt;,
    },

    Array {
        items: Vec&amp;lt;CanonicalValue&amp;gt;,
    },

    Object {
        fields: BTreeMap&amp;lt;String, CanonicalValue&amp;gt;,
    },

    DateTime {
        rfc3339: String,
    },

    Json {
        value: serde_json::Value,
    },
}
```

Separar `Null` de `Absent` é crítico. Em migração PHP para TypeScript, `null`, campo ausente e `undefined` podem representar bugs diferentes.

---

# 11. Modelo canônico de erro

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalError {
    pub kind: ErrorKind,
    pub code: Option&amp;lt;String&amp;gt;,
    pub class: Option&amp;lt;String&amp;gt;,
    pub message: Option&amp;lt;String&amp;gt;,
    pub normalized_message: Option&amp;lt;String&amp;gt;,
    pub http_status: Option&amp;lt;u16&amp;gt;,
    pub retryable: Option&amp;lt;bool&amp;gt;,
    pub frames: Vec&amp;lt;StackFrame&amp;gt;,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Exception,
    Panic,
    Validation,
    Authorization,
    NotFound,
    Conflict,
    Timeout,
    ProcessExit,
    AssertionFailure,
    Unknown,
}
```

Comparação de erro deve ser por policy.

Exemplo:

```toml
[policies.laravel_to_encore_errors]
compare_error_kind = true
compare_error_code = true
compare_http_status = true
message_mode = "regex"
ignore_stack_trace = true
```

Stack trace raramente deve bloquear paridade. Código, tipo lógico, status HTTP e payload de erro costumam importar mais.

---

# 12. Side effects

Side effect é onde a maioria das ferramentas de teste fica míope.

Tipos iniciais:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    DbDelta(DbDelta),
    FileDelta(FileDelta),
    HttpCall(HttpCall),
    QueueMessage(QueueMessage),
    Event(EventEmission),
    Email(EmailEmission),
    CacheOperation(CacheOperation),
    Log(LogRecord),
}
```

## DB delta

```json
{
  "kind": "db_delta",
  "connection": "default",
  "table": "invoices",
  "inserted": [
    {
      "id": "&amp;lt;ID&amp;gt;",
      "customer_id": "cus_123",
      "amount": "199.90",
      "currency": "BRL"
    }
  ],
  "updated": [],
  "deleted": []
}
```

## Mapeamento entre schemas diferentes

A nova implementação pode ter schema diferente. Então precisa existir mapper.

```toml
[effects.db.maps.invoices]
target_table = "billing_invoices"

[effects.db.maps.invoices.fields]
id = "invoice_id"
customer_id = "customer_id"
amount = "total_amount"
currency = "currency_code"
```

Isso permite comparar sem exigir que o banco novo seja uma fotocópia do antigo.

---

# 13. Adapters

## 13.1 Tipos de adapter

```txt
command adapter
http adapter
framework adapter
language SDK
effect probe
report importer
```

## 13.2 Command adapter

O primeiro adapter deve ser genérico.

Ele roda qualquer comando que emita Rewrit Protocol.

```toml
[runtimes.legacy_laravel]
adapter = "command"
command = ["vendor/bin/pest", "--rewrit"]
```

Isso permite integrar qualquer stack sem esperar adapter nativo.

---

## 13.3 HTTP adapter

O segundo adapter mais importante.

Ele sobe duas aplicações, manda a mesma request e compara resposta.

```toml
[runtimes.legacy_laravel.server]
start = ["php", "artisan", "serve", "--port=8001"]
healthcheck = "http://127.0.0.1:8001/health"

[runtimes.encore_ts.server]
start = ["encore", "run", "--listen=127.0.0.1:8002"]
healthcheck = "http://127.0.0.1:8002/health"
```

Use isso cedo para Laravel para Encore, Django para Rust e monolitos HTTP.

Encore tem SDKs/experiência para TypeScript e Go, e a documentação atual posiciona Encore.ts como um SDK open-source de infraestrutura para TypeScript. ([Encore][7])

---

## 13.4 PHP adapter

Camadas:

```txt
rewrit-adapter-php em Rust
sdks/php como Composer package
plugin Pest
extension PHPUnit
helper Laravel
```

Laravel já vem com suporte de testing incluindo Pest e PHPUnit, o que torna esse adapter uma prioridade natural para o caso Laravel para Encore. Pest também tem sistema de plugins e documentação própria para testes. ([Laravel][8])

Exemplo Pest conceitual:

```php
it('creates an invoice', function () {
    $response = $this-&amp;gt;postJson('/api/invoices', [
        'customer_id' =&amp;gt; 'cus_123',
        'amount' =&amp;gt; '199.90',
        'currency' =&amp;gt; 'BRL',
    ]);

    Rewrit::observeHttpResponse($response);
    Rewrit::observeDbDelta('invoices');

    $response-&amp;gt;assertCreated();
})-&amp;gt;rewrit('billing.invoice.create.success');
```

O método `rewrit()` seria adicionado pelo plugin.

---

## 13.5 Node adapter

Camadas:

```txt
rewrit-adapter-node em Rust
sdks/node como npm package
Vitest reporter
Jest reporter
Encore helper
```

Exemplo Vitest conceitual:

```ts
import { test, observeHttpResponse, observeDbDelta } from "@rewrit/vitest";

test.rewrit("billing.invoice.create.success", async ({ client }) =&amp;gt; {
  const response = await client.post("/api/invoices", {
    customer_id: "cus_123",
    amount: "199.90",
    currency: "BRL",
  });

  observeHttpResponse(response);
  await observeDbDelta("billing_invoices");
});
```

---

## 13.6 Python/Django adapter

Camadas:

```txt
rewrit-adapter-python em Rust
pytest plugin
Django helpers
```

Django tem documentação oficial para escrita e execução de testes, e `pytest-django` existe justamente para conectar pytest com projetos Django. ([Django Project][9])

Exemplo conceitual:

```python
import rewrit

@rewrit.case("billing.invoice.create.success")
def test_creates_invoice(client):
    response = client.post("/api/invoices", {
        "customer_id": "cus_123",
        "amount": "199.90",
        "currency": "BRL",
    })

    rewrit.observe_http_response(response)
    rewrit.observe_db_delta("invoices")
```

---

## 13.7 Rust adapter

Para Rust target:

```rust
#[rewrit::case("billing.invoice.create.success")]
#[tokio::test]
async fn creates_invoice() {
    let response = client
        .post("/api/invoices")
        .json(&amp;amp;payload)
        .send()
        .await
        .unwrap();

    rewrit::observe_http_response(response).await;
    rewrit::observe_db_delta("billing_invoices").await;
}
```

O macro pode vir depois. No MVP, um helper explícito já resolve.

---

# 14. Policy engine

A policy define o que é diferença real e o que é ruído.

Exemplo:

```toml
[policies.strict]
mode = "strict"
allow_missing_candidate = false
allow_extra_candidate = true
compare_stdout = false
compare_stderr = false
compare_duration = false

[policies.php_to_ts_api]
mode = "semantic"
allow_integer_float_equivalence = true
allow_header_case_difference = true
allow_object_key_order_difference = true
allow_null_absent_equivalence = false
decimal_as_string = true
ignore_stack_trace = true

[policies.php_to_ts_api.json]
ignore_paths = [
  "$.trace_id",
  "$.generated_at",
  "$.debug"
]

[policies.php_to_ts_api.headers]
ignore = [
  "date",
  "server",
  "x-request-id"
]
```

Regra de ouro:

```txt
strict por padrão
tolerância explícita por path
waiver com motivo e validade
```

Não deixe a policy virar um pano mágico que cobre bug.

---

# 15. Waivers

Waiver é divergência aceita temporariamente.

```toml
[[waivers]]
case = "billing.invoice.cancel.refund_event"
kind = "side_effect_mismatch"
reason = "Encore ainda não publica evento RefundIssued"
owner = "billing-platform"
expires = "2026-08-01"
issue = "BILL-4821"
```

Comportamento:

```txt
waiver válido: não bloqueia, mas aparece no relatório
waiver expirado: bloqueia
waiver sem reason: config inválida
waiver sem expires: config inválida, salvo override explícito
```

---

# 16. Normalizers

Normalizadores removem ruído.

Pipeline exemplo:

```txt
path normalizer
timestamp normalizer
uuid normalizer
http header normalizer
json key order normalizer
php array normalizer
error stack normalizer
decimal normalizer
```

Config:

```toml
[[normalizers]]
kind = "timestamp"
paths = ["$.created_at", "$.updated_at"]
replacement = "&amp;lt;TIMESTAMP&amp;gt;"

[[normalizers]]
kind = "uuid"
paths = ["$.id", "$.items[*].id"]
replacement = "&amp;lt;UUID&amp;gt;"

[[normalizers]]
kind = "http_headers"
lowercase_names = true
sort_values = true

[[normalizers]]
kind = "php_array"
detect_lists = true
```

Importante: todo normalizer aplicado deve aparecer no relatório.

```json
{
  "case_id": "billing.invoice.create.success",
  "normalizers_applied": [
    "http_headers",
    "uuid",
    "timestamp"
  ]
}
```

Isso impede “passou porque apagamos o mundo”.

---

# 17. Comparators

Comparadores recomendados:

```txt
canonical value comparator
json schema comparator
http comparator
error comparator
stdout comparator
stderr comparator
exit code comparator
db delta comparator
event comparator
queue message comparator
file delta comparator
```

Comparator deve retornar uma árvore de diffs:

```rust
pub struct Comparison {
    pub case_id: CaseId,
    pub equivalent: bool,
    pub divergences: Vec&amp;lt;Divergence&amp;gt;,
    pub policy_trace: Vec&amp;lt;PolicyDecision&amp;gt;,
}
```

Cada divergência deve ter:

```txt
case id
path
categoria
severidade
reference
candidate
mensagem humana
mensagem machine-readable
source location
target location
policy aplicada
normalizers aplicados
```

---

# 18. Reports

## 18.1 Terminal

Exemplo:

```txt
Rewrit parity report

Project: billing-migration
Reference: legacy_laravel
Candidate: encore_ts

Cases discovered: 1,351
Cases compared: 1,309
Equivalent: 1,244
Allowed by waiver: 11
Blocking divergences: 54

Parity: 94.81%

Blocking:
  missing_candidate_case: 24
  output_mismatch: 13
  type_mismatch: 8
  side_effect_mismatch: 7
  error_mismatch: 2

Worst suites:
  billing.refunds: 71.40%
  auth.sessions: 84.20%
  orders.checkout: 91.10%

Exit: 1
```

## 18.2 JSON

```json
{
  "schema_version": "rewrit.report.v1",
  "run_id": "01JZ...",
  "project": "billing-migration",
  "reference": "legacy_laravel",
  "candidate": "encore_ts",
  "summary": {
    "cases_discovered": 1351,
    "cases_compared": 1309,
    "equivalent": 1244,
    "waived": 11,
    "blocking": 54,
    "parity_ratio": 0.9481
  },
  "divergences": []
}
```

## 18.3 NDJSON

Para monolitos enormes, NDJSON evita segurar tudo em memória.

## 18.4 JUnit XML

Para CI exibir falhas como testes.

## 18.5 SARIF

Para anotar PRs e permitir que agente localize arquivo/linha.

## 18.6 HTML

Para auditoria executiva e navegação por domínio.

---

# 19. Exit codes

A CLI precisa ser previsível para CI e agentes.

```txt
0  sucesso, paridade alcançada
1  divergências bloqueantes encontradas
2  config, manifest ou contrato inválido
3  discovery falhou
4  adapter indisponível ou incompatível
5  execução de runtime falhou
6  timeout global ou cancelamento
7  erro de escrita de report/artifact
8  nenhum case encontrado quando cases eram obrigatórios
9  feature/policy não suportada
70 erro interno inesperado
```

Diferença importante:

```txt
candidate retornou payload diferente: exit 1
PHP nem conseguiu iniciar: exit 5
rewrit.toml inválido: exit 2
adapter não fala protocol v1: exit 4
```

Isso ajuda agente e pipeline a decidirem o próximo passo.

---

# 20. CLI proposta

```bash
rewrit init --template laravel-to-encore
rewrit doctor
rewrit discover
rewrit audit
rewrit capture --runtime legacy_laravel
rewrit verify --runtime encore_ts
rewrit run --mode mirror
rewrit explain billing.invoice.create.success
rewrit schema export
rewrit report open
```

## `doctor`

Valida ambiente:

```txt
PHP instalado
Composer deps presentes
Node instalado
npm/pnpm/yarn disponível
Encore CLI disponível
conexão com banco de teste
adapters compatíveis
protocol version compatível
```

## `discover`

Lista cases por runtime:

```bash
rewrit discover --runtime legacy_laravel --format json
```

## `audit`

Verifica paridade de existência:

```txt
reference tem 1,351 cases
candidate tem 1,240 cases
missing candidate: 111
orphan candidate: 29
```

## `explain`

Mostra um case com diff rico:

```bash
rewrit explain billing.invoice.create.success
```

Saída:

```txt
Case: billing.invoice.create.success
Suite: billing
Status: failed
Kind: type_mismatch
Path: $.amount

Reference:
  "199.90"

Candidate:
  199.9

Policy:
  decimal_as_string = true

Hint:
  Retorne amount como string decimal com duas casas, não como number.
```

---

# 21. Design patterns usados

## 21.1 Hexagonal Architecture

Core não depende de frameworks.

```txt
core
  recebe abstrações
adapters
  conhecem o mundo sujo
```

## 21.2 Strategy

Comparadores, normalizadores e reporters são strategies.

```rust
Box&amp;lt;dyn Comparator&amp;gt;
Box&amp;lt;dyn Normalizer&amp;gt;
Box&amp;lt;dyn Reporter&amp;gt;
```

## 21.3 Chain of Responsibility

Normalizers rodam em pipeline.

```txt
raw observation
  ↓
path normalizer
  ↓
timestamp normalizer
  ↓
uuid normalizer
  ↓
http normalizer
  ↓
normalized observation
```

## 21.4 Registry/Factory

Adapters e reporters registrados por nome.

```rust
registry.register_adapter("php:pest", PestAdapter::factory());
registry.register_adapter("node:vitest", VitestAdapter::factory());
registry.register_reporter("junit", JunitReporter::factory());
```

## 21.5 Snapshot/Golden Master

Baseline mode usa golden master versionado.

Snapshot testing é bem adequado quando valores de referência são grandes ou mudam com revisão controlada, e a crate `insta` documenta esse estilo para Rust. ([Docs.rs][10])

## 21.6 Event-driven interno

Engine emite eventos:

```txt
RunStarted
CaseDiscovered
CaseStarted
ObservationReceived
CaseCompared
DivergenceFound
RunFinished
```

Reporters escutam eventos. Isso evita acoplamento.

---

# 22. Test strategy da própria lib

## 22.1 Unit tests

Foco:

```txt
CanonicalValue equality
normalizers
policy engine
waivers
schema validation
diff generation
exit code resolver
manifest parser
```

Exemplos:

```txt
normalizes_uuid_at_configured_path
does_not_normalize_uuid_outside_path
treats_null_and_absent_as_different_by_default
allows_null_absent_only_when_policy_says_so
detects_decimal_string_vs_float_mismatch
detects_missing_candidate_case
fails_expired_waiver
```

## 22.2 Integration tests

Foco:

```txt
fake adapters
process runner
timeout handling
NDJSON protocol
report generation
baseline store
```

Fixtures:

```txt
tests/fixtures/fake-adapters/pass
tests/fixtures/fake-adapters/output-mismatch
tests/fixtures/fake-adapters/timeout
tests/fixtures/fake-adapters/malformed-json
tests/fixtures/fake-adapters/missing-case
```

## 22.3 Snapshot tests

Use para:

```txt
terminal report
JSON report
SARIF report
JUnit report
diff rendering
explain output
```

## 22.4 E2E tests

Cenários mínimos:

```txt
command-to-command: shell scripts emitindo observations
http-to-http: dois servidores fake
php-to-node: fixtures pequenas
django-to-rust: fixtures pequenas
```

## 22.5 Property tests

Use para invariantes de comparação:

```txt
compare(a, a) sempre equivalente após normalização determinística
normalização deve ser idempotente
ordem de object keys não muda resultado
waiver expirado nunca permite blocking divergence
```

---

# 23. MVP viável

Não comece com todos os adapters. Comece com o núcleo que prova a tese.

## MVP 1: protocolo e engine mínima

Entregáveis:

```txt
rewrit-model
rewrit-core
rewrit-engine
rewrit-cli
rewrit-protocol
rewrit-report
command adapter
JSON report
terminal report
strict comparator
manifest parser
exit codes
```

Critério de aceite:

```txt
dois scripts em qualquer linguagem emitem observations
rewrit compara por case_id
detecta pass, mismatch, missing case e timeout
gera JSON report
retorna exit code correto
```

## MVP 2: HTTP adapter

Entregáveis:

```txt
start/stop de servidores
healthcheck
requests declaradas em contrato
comparação de status, headers e body JSON
normalização de headers, timestamps e ids
```

Critério de aceite:

```txt
comparar uma API Laravel fake com uma API Node fake
identificar diferença de status
identificar diferença de schema
identificar diferença de tipo
```

## MVP 3: Laravel para Encore

Entregáveis:

```txt
template laravel-to-encore
PHP SDK mínimo
Node SDK mínimo
Pest integration mínima
Vitest integration mínima
case_id obrigatório
audit de missing candidate
baseline mode
```

Critério de aceite:

```txt
um projeto Laravel exemplo gera baseline
um projeto Encore/TS exemplo emite observations equivalentes
rewrit detecta teste ausente no Encore
rewrit detecta payload incompatível
rewrit gera JUnit e JSON
```

## MVP 4: Django para Rust

Entregáveis:

```txt
pytest plugin mínimo
Rust SDK mínimo
cargo test adapter básico
HTTP-first migration guide
```

Critério de aceite:

```txt
Django reference e Rust candidate comparados por contratos HTTP
case_id consistente
reports úteis para agente corrigir divergências
```

---

# 24. Caso Laravel para Encore

Assumindo que “lavável” significa Laravel.

## Estratégia correta

Não tente converter todos os testes Pest/PHPUnit diretamente para Vitest.

Faça:

```txt
1. Identificar domínios
2. Definir case_ids estáveis
3. Capturar comportamento observável do Laravel
4. Gerar baseline
5. Criar testes equivalentes no Encore com mesmos case_ids
6. Comparar observations
7. Bloquear CI enquanto houver divergência não aceita
```

## Ordem de migração recomendada

```txt
HTTP endpoints
commands
jobs
domain services puros
side effects de banco
eventos e filas
casos internos mais acoplados
```

## Exemplo de suite

```toml
[[suites]]
id = "auth"
policy = "http_api_strict"

[[suites]]
id = "billing"
policy = "money_strict"

[[suites]]
id = "orders"
policy = "side_effects_strict"
```

## Regra de ouro para dinheiro

```txt
dinheiro não é float
dinheiro é decimal canônico
preferencialmente string decimal no contrato
```

Exemplo:

```json
{
  "amount": "199.90"
}
```

Não aceite `199.9` em TypeScript como equivalente por padrão.

---

# 25. Caso Django para Rust

## Estratégia correta

Use boundary contracts primeiro.

```txt
Django view/API
  ↓
HTTP contract
  ↓
Rust service/API
```

Depois entre no domínio.

Para Rust, vale usar helpers que emitem observations diretamente. Para Django, plugin pytest.

Modelo:

```txt
pytest-django case_id
        ↓
observation reference
        ↓
Rust test com mesmo case_id
        ↓
observation candidate
        ↓
comparison
```

---

# 26. Garantir que os mesmos testes existam

Isso é uma feature própria: **case binding audit**.

Regra:

```txt
todo required case da reference precisa ter candidate case equivalente
```

Manifest:

```toml
[[bindings]]
case = "auth.login.success"
reference = "tests/Feature/Auth/LoginTest.php::login_success"
candidate = "tests/auth/login.test.ts::login success"
required = true
```

Ou por ID direto:

```toml
[[bindings]]
case = "auth.login.success"
required = true
```

Se o candidate não emitir observation com esse `case_id`:

```txt
missing_candidate_case
exit 1
```

Se o candidate tiver teste extra:

```txt
orphan_candidate_case
warning por padrão
```

Pode virar erro:

```toml
[policy.audit]
fail_on_orphan_candidate = true
```

---

# 27. Reports para agentes de código

A lib será usada por humanos e AI. Então o report precisa ser delicioso para máquina.

Cada divergência deve incluir:

```txt
case_id
suite
kind
severity
source file/line
target file/line
expected
actual
json path
policy
normalizers applied
minimal reproduction
suggested next action
```

Exemplo:

```json
{
  "case_id": "auth.login.invalid_password",
  "kind": "error_mismatch",
  "severity": "blocking",
  "source_location": {
    "path": "tests/Feature/Auth/LoginTest.php",
    "line": 42
  },
  "target_location": {
    "path": "tests/auth/login.test.ts",
    "line": 18
  },
  "expected": {
    "http_status": 422,
    "error_code": "INVALID_CREDENTIALS"
  },
  "actual": {
    "http_status": 401,
    "error_code": "UNAUTHORIZED"
  },
  "hint": "Alinhar status e código de erro do candidate ao contrato da reference."
}
```

A lib não usa AI. Ela produz munição limpa para AI.

---

# 28. Segurança e isolamento

Essa ferramenta executa código de projetos. Portanto:

```txt
assumir repositório confiável por padrão
não vazar env vars em reports
redigir secrets automaticamente
timeouts obrigatórios
kill de process tree
cwd explícito
env allowlist
network control configurável
temp dir por execução
reports sem tokens
```

Config:

```toml
[security]
redact_env = true
redact_patterns = [
  "sk_live_[A-Za-z0-9]+",
  "Bearer [A-Za-z0-9._-]+"
]

[runner]
kill_process_tree = true
default_timeout_ms = 30000
max_stdout_bytes = 1048576
max_stderr_bytes = 1048576
```

Sandbox pesado com Docker/Podman pode vir depois. Não coloque isso como pré-requisito do MVP, senão o projeto vira plataforma de containers antes de virar parity engine.

---

# 29. Dependências Rust sugeridas

```toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
schemars = "1"
clap = { version = "4", features = ["derive"] }
thiserror = "2"
miette = "7"
tokio = { version = "1", features = ["process", "macros", "rt-multi-thread", "time", "fs", "io-util"] }
tracing = "0.1"
tracing-subscriber = "0.3"
camino = "1"
globset = "0.4"
ignore = "0.4"
indexmap = { version = "2", features = ["serde"] }
similar = "2"
tempfile = "3"
uuid = { version = "1", features = ["v7", "serde"] }
time = { version = "0.3", features = ["serde"] }
```

Princípio:

```txt
rewrit-model deve ser leve
rewrit-core deve ter poucas dependências
rewrit-engine pode usar tokio
rewrit-cli pode usar miette/clap/tracing
adapters ficam em crates separados
```

---

# 30. Qualidade open-source

Rust API Guidelines recomendam documentação de crate, exemplos e APIs consistentes. Para esse projeto, isso não é cosmético, é sobrevivência, porque adapters externos e agentes vão depender de contratos estáveis. ([Rust Lang][11])

Checklist:

```txt
README com quickstart real
docs de protocolo
examples executáveis
rustdoc em APIs públicas
sem unsafe no core
sem panics em paths esperados
erros tipados em libs
diagnósticos bonitos na CLI
semver sério
MSRV declarado
CHANGELOG
CONTRIBUTING
SECURITY
dual license MIT/Apache-2.0
CI com fmt, clippy, tests, docs
release automatizado
schemas versionados
ADRs para decisões críticas
```

CI mínimo:

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace --all-features
      - run: cargo doc --workspace --all-features --no-deps
```

---

# 31. Anti-patterns a evitar

```txt
1. Core conhecendo Laravel, Django ou Vitest
2. Parsing de stdout humano como contrato
3. Comparação por string bruta como default
4. Normalização ampla demais
5. Retry escondendo flakiness
6. Waiver eterno
7. Um adapter gigante para todas as linguagens
8. Tentar fazer AST parser universal
9. Tentar garantir side effects sem probes claros
10. Usar AI dentro da lib
11. Chamar tudo de “teste falhou”
12. Misturar erro de infra com divergência semântica
13. Não versionar protocolo
14. Não diferenciar null de absent
15. Tratar dinheiro como float
```

---

# 32. Arquitetura visual

```txt
                 rewrit.toml
                     │
                     ▼
              ┌─────────────┐
              │   Engine    │
              └──────┬──────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
   Discovery      Runner       Store
        │            │            │
        ▼            ▼            ▼
     Cases     Observations   Baselines
        │            │            │
        └──────┬─────┴─────┬──────┘
               ▼           ▼
        ┌─────────────┐ ┌─────────────┐
        │ Normalizer  │ │ Validator   │
        └──────┬──────┘ └──────┬──────┘
               ▼               ▼
             ┌───────────────────┐
             │    Comparator     │
             └─────────┬─────────┘
                       ▼
             ┌───────────────────┐
             │   Policy Engine   │
             └─────────┬─────────┘
                       ▼
             ┌───────────────────┐
             │     Reporters     │
             └─────────┬─────────┘
                       ▼
                  Exit Code
```

---

# 33. Primeiros arquivos que eu criaria

Ordem prática:

```txt
1. crates/rewrit-model
2. crates/rewrit-protocol
3. crates/rewrit-core
4. crates/rewrit-engine
5. crates/rewrit-report
6. crates/rewrit-cli
7. examples/command-to-command
8. examples/http-to-http
9. docs/protocol/adapter-protocol-v1.md
10. docs/adr/0001-ndjson-adapter-protocol.md
```

Não comece pelo adapter Laravel. Comece pelo protocolo. O adapter Laravel será muito melhor quando tiver uma pista de pouso.

---

# 34. O coração do sistema

O coração é este contrato:

```rust
pub trait Adapter: Send + Sync {
    fn id(&amp;amp;self) -&amp;gt; AdapterId;

    fn doctor(
        &amp;amp;self,
        ctx: DoctorContext,
    ) -&amp;gt; BoxFuture&amp;lt;'_, Result&amp;lt;DoctorReport, AdapterError&amp;gt;&amp;gt;;

    fn discover(
        &amp;amp;self,
        ctx: DiscoveryContext,
    ) -&amp;gt; BoxFuture&amp;lt;'_, Result&amp;lt;Vec&amp;lt;Case&amp;gt;, AdapterError&amp;gt;&amp;gt;;

    fn run(
        &amp;amp;self,
        ctx: RunContext,
    ) -&amp;gt; BoxStream&amp;lt;'_, Result&amp;lt;AdapterEvent, AdapterError&amp;gt;&amp;gt;;
}
```

E o protocolo externo equivalente:

```txt
adapter discover
adapter run
adapter doctor
```

Entrada e saída sempre versionadas.

```json
{
  "schema_version": "rewrit.adapter_request.v1",
  "command": "run",
  "runtime_id": "legacy_laravel",
  "cases": ["billing.invoice.create.success"]
}
```

---

# 35. Definição de sucesso do projeto

A lib está fazendo o trabalho certo quando consegue responder com precisão:

```txt
Quais contratos existem na referência?
Quais existem no candidato?
Quais estão equivalentes?
Quais divergiram?
Qual tipo de divergência?
Qual arquivo provavelmente precisa mudar?
Qual diferença é ruído?
Qual diferença está aceita temporariamente?
Qual diferença bloqueia merge?
Qual é a taxa de paridade por domínio?
```

Para o caso Laravel para Encore, o objetivo operacional é:

```txt
nenhum endpoint migrado entra sem case_id
nenhum case_id reference fica sem candidate
nenhuma divergência blocking passa no CI
todo waiver tem dono, motivo e validade
todo report é legível por humano e agente
```

---

# 36. Veredito arquitetural

Eu faria a lib como:

```txt
Rust core
CLI first
NDJSON protocol
adapters externos
contracts em JSON Schema
reports multi-formato
policies explícitas
waivers expirantes
HTTP adapter cedo
Laravel/Encore como primeiro showcase real
Django/Rust como segundo showcase
```

O pulo do gato: **não comparar frameworks, comparar observações canônicas**.

Framework é sotaque. Contrato é idioma.

[1]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html?utm_source=chatgpt.com "Cargo Workspaces - The Rust Programming Language"
[2]: https://serde.rs/?utm_source=chatgpt.com "Overview · Serde"
[3]: https://doc.rust-lang.org/cargo/guide/project-layout.html?utm_source=chatgpt.com "Package Layout - The Cargo Book"
[4]: https://testanything.org/?utm_source=chatgpt.com "Test Anything Protocol: Home"
[5]: https://github.com/testmoapp/junitxml?utm_source=chatgpt.com "Common JUnit XML Format &amp;amp; Examples"
[6]: https://docs.rs/clap?utm_source=chatgpt.com "clap - Rust"
[7]: https://encore.dev/?utm_source=chatgpt.com "Backend Infrastructure for Humans and Agents — Encore"
[8]: https://laravel.com/docs/13.x/testing?utm_source=chatgpt.com "Testing: Getting Started | Laravel 13.x - The clean stack for ..."
[9]: https://docs.djangoproject.com/en/6.0/topics/testing/overview/?utm_source=chatgpt.com "Writing and running tests"
[10]: https://docs.rs/insta?utm_source=chatgpt.com "insta - Rust"
[11]: https://rust-lang.github.io/api-guidelines/checklist.html?utm_source=chatgpt.com "Rust API Guidelines Checklist"
