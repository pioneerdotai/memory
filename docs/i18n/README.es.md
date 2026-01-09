<img width="2000" height="491" alt="Social Cover (6)" src="https://github.com/user-attachments/assets/4e256804-53ac-4173-bcff-81994d52bf5c" />

<p align="center">
  <strong>Memvid es una capa de memoria de un solo archivo para agentes de IA, con recuperación instantánea y memoria a largo plazo.</strong><br/>
  Memoria persistente, versionada y portable, sin bases de datos.
</p>

<p align="center">
  <a href="https://www.memvid.com">Sitio web</a>
  ·
  <a href="https://sandbox.memvid.com">Probar Sandbox</a>
  ·
  <a href="https://docs.memvid.com">Docs</a>
  ·
  <a href="https://github.com/memvid/memvid/discussions">Discussions</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/memvid-core"><img src="https://img.shields.io/crates/v/memvid-core?style=flat-square&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/memvid-core"><img src="https://img.shields.io/docsrs/memvid-core?style=flat-square&logo=docs.rs" alt="docs.rs" /></a>
  <a href="https://github.com/memvid/memvid/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <a href="https://github.com/memvid/memvid/stargazers"><img src="https://img.shields.io/github/stars/memvid/memvid?style=flat-square&logo=github" alt="Stars" /></a>
  <a href="https://github.com/memvid/memvid/network/members"><img src="https://img.shields.io/github/forks/memvid/memvid?style=flat-square&logo=github" alt="Forks" /></a>
  <a href="https://github.com/memvid/memvid/issues"><img src="https://img.shields.io/github/issues/memvid/memvid?style=flat-square&logo=github" alt="Issues" /></a>
  <a href="https://discord.gg/2mynS7fcK7"><img src="https://img.shields.io/discord/1442910055233224745?style=flat-square&logo=discord&label=discord" alt="Discord" /></a>
</p>

<p align="center">
    <a href="https://trendshift.io/repositories/17293" target="_blank"><img src="https://trendshift.io/api/badge/repositories/17293" alt="memvid%2Fmemvid | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/</a>
</p>

<h2 align="center">⭐️ Deja una STAR para apoyar el proyecto ⭐️</h2>
</p>

## ¿Qué es Memvid?

Memvid es un sistema de memoria portable para IA que empaqueta tus datos, embeddings, estructura de búsqueda y metadatos en un solo archivo.

En lugar de ejecutar pipelines RAG complejos o bases de datos vectoriales basadas en servidor, Memvid permite una recuperación rápida directamente desde el archivo.

El resultado es una capa de memoria agnóstica al modelo, sin infraestructura, que da a los agentes de IA una memoria persistente y a largo plazo que pueden llevar a cualquier parte.

---

## ¿Por qué fotogramas de vídeo?

Memvid se inspira en la codificación de vídeo, no para almacenar vídeo, sino para **organizar la memoria de IA como una secuencia de Smart Frames ultrarrápida y append-only.**

Un Smart Frame es una unidad inmutable que almacena contenido junto con marcas de tiempo (timestamps), checksums y metadatos básicos.
Los frames se agrupan de una forma que permite una compresión, indexación y lecturas paralelas eficientes.

Este diseño basado en frames permite:

-   Escrituras append-only sin modificar ni corromper los datos existentes
-   Consultas sobre estados pasados de la memoria
-   Inspección estilo línea temporal (timeline) de cómo evoluciona el conocimiento
-   Seguridad ante fallos (crash safety) mediante frames confirmados e inmutables
-   Compresión eficiente usando técnicas adaptadas de la codificación de vídeo

El resultado es un único archivo que se comporta como una línea temporal de memoria “rebobinable” para sistemas de IA.

---

## Conceptos principales

-   **Living Memory Engine**
    Añade, ramifica (branch) y evoluciona la memoria de forma continua entre sesiones.

-   **Capsule Context (`.mv2`)**
    Cápsulas de memoria autocontenidas y compartibles, con reglas y caducidad.

-   **Time-Travel Debugging**
    Rebobina, reproduce (replay) o ramifica cualquier estado de memoria.

-   **Smart Recall**
    Acceso local a memoria en menos de 5ms con caché predictiva.

-   **Codec Intelligence**
    Selecciona y actualiza la compresión automáticamente con el tiempo.

---

## Casos de uso

Memvid es una capa de memoria portable y serverless que da a los agentes de IA memoria persistente y recuerdo rápido. Como es agnóstica al modelo, multi-modal y funciona totalmente offline, los desarrolladores están usando Memvid en una amplia gama de aplicaciones reales.

-   Agentes de IA de larga duración
-   Bases de conocimiento empresariales
-   Sistemas de IA offline-first
-   Comprensión de codebases
-   Agentes de soporte al cliente
-   Automatización de flujos de trabajo
-   Copilotos de ventas y marketing
-   Asistentes de conocimiento personal
-   Agentes médicos, legales y financieros
-   Flujos de trabajo de IA auditables y depurables
-   Aplicaciones personalizadas

---

## SDKs & CLI

Usa Memvid en tu lenguaje preferido:

| Package         | Install                     | Links                                                                                                               |
| --------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **CLI**         | `npm install -g memvid-cli` | [![npm](https://img.shields.io/npm/v/memvid-cli?style=flat-square)](https://www.npmjs.com/package/memvid-cli)       |
| **Node.js SDK** | `npm install @memvid/sdk`   | [![npm](https://img.shields.io/npm/v/@memvid/sdk?style=flat-square)](https://www.npmjs.com/package/@memvid/sdk)     |
| **Python SDK**  | `pip install memvid-sdk`    | [![PyPI](https://img.shields.io/pypi/v/memvid-sdk?style=flat-square)](https://pypi.org/project/memvid-sdk/)         |
| **Rust**        | `cargo add memvid-core`     | [![Crates.io](https://img.shields.io/crates/v/memvid-core?style=flat-square)](https://crates.io/crates/memvid-core) |

---

## Instalación (Rust)

### Requisitos

-   **Rust 1.85.0+** — Instálalo desde [rustup.rs](https://rustup.rs)

### Añadir a tu proyecto

```toml
[dependencies]
memvid-core = "2.0"
```

### Feature Flags

| Feature             | Description                                    |
| ------------------- | ---------------------------------------------- |
| `lex`               | Full-text search with BM25 ranking (Tantivy)   |
| `pdf_extract`       | Pure Rust PDF text extraction                  |
| `vec`               | Vector similarity search (HNSW + ONNX)         |
| `clip`              | CLIP visual embeddings for image search        |
| `whisper`           | Audio transcription with Whisper               |
| `temporal_track`    | Natural language date parsing ("last Tuesday") |
| `parallel_segments` | Multi-threaded ingestion                       |
| `encryption`        | Password-based encryption capsules (.mv2e)     |

Activa las features según lo necesites:

```toml
[dependencies]
memvid-core = { version = "2.0", features = ["lex", "vec", "temporal_track"] }
```

---

## Inicio rápido

```rust
use memvid_core::{Memvid, PutOptions, SearchRequest};

fn main() -> memvid_core::Result<()> {
    // Create a new memory file
    let mut mem = Memvid::create("knowledge.mv2")?;

    // Add documents with metadata
    let opts = PutOptions::builder()
        .title("Meeting Notes")
        .uri("mv2://meetings/2024-01-15")
        .tag("project", "alpha")
        .build();
    mem.put_bytes_with_options(b"Q4 planning discussion...", opts)?;
    mem.commit()?;

    // Search
    let response = mem.search(SearchRequest {
        query: "planning".into(),
        top_k: 10,
        snippet_chars: 200,
        ..Default::default()
    })?;

    for hit in response.hits {
        println!("{}: {}", hit.title.unwrap_or_default(), hit.text);
    }

    Ok(())
}
```

---

## Build

Clona el repositorio:

```bash
git clone https://github.com/memvid/memvid.git
cd memvid
```

Compila en modo debug:

```bash
cargo build
```

Compila en modo release (optimizado):

```bash
cargo build --release
```

Compila con features específicas:

```bash
cargo build --release --features "lex,vec,temporal_track"
```

---

## Ejecutar tests

Ejecuta todos los tests:

```bash
cargo test
```

Ejecuta los tests con salida:

```bash
cargo test -- --nocapture
```

Ejecuta un test específico:

```bash
cargo test test_name
```

Ejecuta solo tests de integración:

```bash
cargo test --test lifecycle
cargo test --test search
cargo test --test mutation
```

---

## Ejemplos

El directorio `examples/` contiene ejemplos funcionales:

### Uso básico

Demuestra operaciones de create, put, search y timeline:

```bash
cargo run --example basic_usage
```

### Ingesta de PDF

Ingiere y busca documentos PDF (usa el paper “Attention Is All You Need”):

```bash
cargo run --example pdf_ingestion
```

### Búsqueda visual con CLIP

Búsqueda de imágenes usando embeddings de CLIP (requiere la feature `clip`):

```bash
cargo run --example clip_visual_search --features clip
```

### Transcripción con Whisper

Transcripción de audio (requiere la feature `whisper`):

```bash
cargo run --example test_whisper --features whisper
```

---

## Formato de archivo

Todo vive en un único archivo `.mv2`:

```
┌────────────────────────────┐
│ Header (4KB)               │  Magic, version, capacity
├────────────────────────────┤
│ Embedded WAL (1-64MB)      │  Crash recovery
├────────────────────────────┤
│ Data Segments              │  Compressed frames
├────────────────────────────┤
│ Lex Index                  │  Tantivy full-text
├────────────────────────────┤
│ Vec Index                  │  HNSW vectors
├────────────────────────────┤
│ Time Index                 │  Chronological ordering
├────────────────────────────┤
│ TOC (Footer)               │  Segment offsets
└────────────────────────────┘
```

Sin archivos `.wal`, `.lock`, `.shm` ni sidecars. Nunca.

Consulta [MV2_SPEC.md](MV2_SPEC.md) para la especificación completa del formato de archivo.

---

## Soporte

¿Tienes preguntas o feedback?
Email: contact@memvid.com

**Deja una ⭐ para mostrar apoyo**

---

## Licencia

Apache License 2.0 — consulta el archivo [LICENSE](LICENSE) para más detalles.


