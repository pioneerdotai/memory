<img width="2000" height="491" alt="Social Cover (6)" src="https://github.com/user-attachments/assets/4e256804-53ac-4173-bcff-81994d52bf5c" />

<p align="center">
  <strong>Memvid è un layer di memoria a singolo file per agenti AI con recupero istantaneo e memoria a lungo termine.</strong><br/>
  Memoria persistente, versionata e portabile, senza database.
</p>

<p align="center">
  <a href="https://www.memvid.com">Sito Web</a>
  ·
  <a href="https://sandbox.memvid.com">Prova Sandbox</a>
  ·
  <a href="https://docs.memvid.com">Documentazione</a>
  ·
  <a href="https://github.com/memvid/memvid/discussions">Discussioni</a>
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

<h2 align="center">⭐️ Lascia una STELLA per supportare il progetto ⭐️</h2>
</p>

## Cos'è Memvid?

Memvid è un sistema di memoria AI portatile che impacchetta i tuoi dati, embeddings, struttura di ricerca e metadati in un singolo file.

Invece di eseguire complesse pipeline RAG o database vettoriali server-based, Memvid consente un recupero veloce direttamente dal file.

Il risultato è un layer di memoria indipendente dal modello (model-agnostic), privo di infrastruttura, che fornisce agli agenti AI una memoria persistente a lungo termine che possono portare ovunque.

---

## Perché i Frame Video?

Memvid trae ispirazione dalla codifica video, non per memorizzare video, ma per **organizzare la memoria dell'AI come una sequenza di Smart Frame in sola aggiunta (append-only) e ultra-efficiente.**

Uno Smart Frame è un'unità immutabile che memorizza il contenuto insieme a timestamp, checksum e metadati di base.
I frame sono raggruppati in un modo da consentire compressione, indicizzazione e letture parallele efficienti.

Questo design basato sui frame consente:

-   Scritture in sola aggiunta (append-only) senza modificare o corrompere i dati esistenti
-   Query sugli stati passati della memoria
-   Ispezione in stile timeline di come la conoscenza evolve
-   Sicurezza contro i crash grazie a frame immutabili e consolidati
-   Compressione efficiente utilizzando tecniche adattate dalla codifica video

Il risultato è un singolo file che si comporta come una timeline di memoria riavvolgibile per sistemi AI.

---

## Concetti Fondamentali

-   **Motore di Memoria Vivente**
    Accoda, dirama ed evolvi continuamente la memoria attraverso le sessioni.

-   **Contesto a Capsula (`.mv2`)**
    Capsule di memoria autosufficienti e condivisibili con regole e scadenza.

-   **Debugging Time-Travel**
    Riavvolgi, riproduci o dirama qualsiasi stato della memoria.

-   **Smart Recall**
    Accesso alla memoria locale sotto i 5ms con caching predittivo.

-   **Intelligenza Codec**
    Seleziona automaticamente e aggiorna la compressione nel tempo.

---

## Casi d'Uso

Memvid è un layer di memoria portatile e serverless che fornisce agli agenti AI memoria persistente e recupero rapido delle informazioni. Poiché è indipendente dal modello (model-agnostic), multimodale e funziona completamente offline, gli sviluppatori stanno usando Memvid in una vasta gamma di applicazioni del mondo reale.

-   Agenti AI a Lunga Esecuzione
-   Knowledge Base Aziendali
-   Sistemi AI Offline-First
-   Comprensione di Codebase
-   Agenti di Supporto Clienti
-   Automazione dei Workflow
-   Copilots per Vendite e Marketing
-   Assistenti di Conoscenza Personale
-   Agenti Medici, Legali e Finanziari
-   Workflow AI Verificabili e Debuggabili
-   Applicazioni Personalizzate

---

## SDK & CLI

Usa Memvid nel tuo linguaggio preferito:

| Pacchetto | Installazione | Link |
| --------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **CLI** | `npm install -g memvid-cli` | [![npm](https://img.shields.io/npm/v/memvid-cli?style=flat-square)](https://www.npmjs.com/package/memvid-cli) |
| **Node.js SDK** | `npm install @memvid/sdk` | [![npm](https://img.shields.io/npm/v/@memvid/sdk?style=flat-square)](https://www.npmjs.com/package/@memvid/sdk) |
| **Python SDK** | `pip install memvid-sdk` | [![PyPI](https://img.shields.io/pypi/v/memvid-sdk?style=flat-square)](https://pypi.org/project/memvid-sdk/) |
| **Rust** | `cargo add memvid-core` | [![Crates.io](https://img.shields.io/crates/v/memvid-core?style=flat-square)](https://crates.io/crates/memvid-core) |

---

## Installazione (Rust)

### Requisiti

-   **Rust 1.85.0+** — Installa da [rustup.rs](https://rustup.rs)

### Aggiungi al Tuo Progetto

```toml
[dependencies]
memvid-core = "2.0"
```

### Feature Flags

| Feature | Descrizione |
| ------------------- | ---------------------------------------------- |
| `lex` | Ricerca full-text con ranking BM25 (Tantivy) |
| `pdf_extract` | Estrazione testo PDF in puro Rust |
| `vec` | Ricerca per similarità vettoriale (HNSW + ONNX) |
| `clip` | Visual embeddings CLIP per ricerca immagini |
| `whisper` | Trascrizione audio con Whisper |
| `temporal_track` | Parsing date in linguaggio naturale ("last Tuesday") |
| `parallel_segments` | Ingestione multi-thread |
| `encryption` | Capsule crittografate basate su password (.mv2e) |

Abilita le feature secondo necessità:

```toml
[dependencies]
memvid-core = { version = "2.0", features = ["lex", "vec", "temporal_track"] }
```

---

## Avvio Rapido

```rust
use memvid_core::{Memvid, PutOptions, SearchRequest};

fn main() -> memvid_core::Result<()> {
    // Crea un nuovo file di memoria
    let mut mem = Memvid::create("knowledge.mv2")?;

    // Aggiungi documenti con metadati
    let opts = PutOptions::builder()
        .title("Meeting Notes")
        .uri("mv2://meetings/2024-01-15")
        .tag("project", "alpha")
        .build();
    mem.put_bytes_with_options(b"Q4 planning discussion...", opts)?;
    mem.commit()?;

    // Cerca
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

Clona il repository:

```bash
git clone https://github.com/memvid/memvid.git
cd memvid
```

Compila in modalità debug:

```bash
cargo build
```

Compila in modalità release (ottimizzata):

```bash
cargo build --release
```

Compila con feature specifiche:

```bash
cargo build --release --features "lex,vec,temporal_track"
```

---

## Eseguire i Test

Esegui tutti i test:

```bash
cargo test
```

Esegui i test con output:

```bash
cargo test -- --nocapture
```

Esegui un test specifico:

```bash
cargo test test_name
```

Esegui solo i test di integrazione:

```bash
cargo test --test lifecycle
cargo test --test search
cargo test --test mutation
```

---

## Esempi

La cartella `examples/` contiene esempi funzionanti:

### Utilizzo Base

Dimostra le operazioni di creazione, inserimento, ricerca e timeline:

```bash
cargo run --example basic_usage
```

### Ingestione PDF

Ingerisci e cerca documenti PDF (usa il paper "Attention Is All You Need"):

```bash
cargo run --example pdf_ingestion
```

### Ricerca Visuale CLIP

Ricerca immagini usando embeddings CLIP (richiede la feature `clip`):

```bash
cargo run --example clip_visual_search --features clip
```

### Trascrizione Whisper

Trascrizione audio (richiede la feature `whisper`):

```bash
cargo run --example test_whisper --features whisper
```

---

## Formato del File

Tutto risiede in un singolo file `.mv2`:

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

Nessun file `.wal`, `.lock`, `.shm` o file di supporto. Mai.

Vedi [MV2_SPEC.md](MV2_SPEC.md) per la specifica completa del formato del file.

---

## Supporto

Hai domande o feedback?
Email: contact@memvid.com

**Lascia una ⭐ per mostrare supporto**

---

## Licenza

Apache License 2.0 — vedi il file [LICENSE](LICENSE) per i dettagli.
