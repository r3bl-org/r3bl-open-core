<!-- cspell:words CRDT epoll zeroconf myapp lamport syncthing mdns zeroconf zstd socat -->

# Resilient Reactor Thread (RRT) Applications

## Overview

This document captures applications of the **Resilient Reactor Thread (RRT)** pattern for building
distributed, peer-to-peer applications using dedicated blocking threads with channel-based
communication.

**Status**: Design exploration / Future work

**Related**: See `task/introduce-resilient-reactor-thread.md` for the implementation plan to extract
reusable RRT infrastructure from `mio_poller`. Once RRT is complete, the use cases below become much
easier to implement.

## The Core Pattern

```text
┌─────────────────────────────────┐              ┌──────────────────────────┐
│ Blocking Resource Thread        │   channel    │ Async Consumers          │
│ ─────────────────────────────── │ ──────────▶  │ ──────────────────────── │
│ • Exclusively owns fds/sockets  │              │ • React to events        │
│ • Blocks on poll() - zero CPU   │ ◀──────────  │ • Send commands back     │
│ • Lives for process lifetime    │  (optional)  │ • Business logic         │
└─────────────────────────────────┘              └──────────────────────────┘
```

### Why This Pattern Works

| Benefit                         | Explanation                                                            |
| :------------------------------ | :--------------------------------------------------------------------- |
| **Clear ownership**             | Each thread exclusively owns its resources (fds, sockets)—no races     |
| **Efficient blocking**          | mio/epoll uses zero CPU while waiting                                  |
| **Clean boundaries**            | Channels provide API isolation between blocking I/O and business logic |
| **Coordination in async layer** | Business logic stays in tokio; blocking stays isolated                 |

### Existing Implementations

#### 1. Terminal Input (Dedicated Reactor Thread)

The `mio_poller` module (`tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/`)
implements the blocking thread pattern:

- **Resources**: stdin fd, SIGWINCH signal fd
- **Blocking**: `mio::Poll::poll()` waits for input or resize signals
- **Channel**: `tokio::sync::broadcast` fans out events to async consumers

> **Note**: This implementation is being refactored into the generic **Resilient Reactor Thread
> (RRT)** pattern. See `task/introduce-resilient-reactor-thread.md` for details.

#### 2. Length-Prefixed Bincode Protocol (Network I/O)

The `network_io` module (`tui/src/network_io/`) implements the serialization protocol:

| File                        | Purpose                                           |
| :-------------------------- | :------------------------------------------------ |
| `length_prefix_protocol.rs` | Length-prefixed framing + handshake + timeouts    |
| `bincode_serde.rs`          | Bincode v2.x serialization (Serde-based)          |
| `compress.rs`               | Compression layer                                 |
| `protocol_types.rs`         | Type aliases (`LengthPrefixType = u64`, `Buffer`) |

**Protocol stack:**

```text
byte_io::try_write/try_read  →  compress  →  bincode_serde  →  Rust structs
     (u64 length prefix)        (zstd?)       (binary)
```

**Features:**

- Magic number handshake (`0xACED_FACE_BABE_CAFE`) + protocol version
- Max payload validation (10 MB)
- Timeout handling
- Works with any `AsyncRead + AsyncWrite` (TCP, Unix, TLS streams)

**See also:** [rust-scratch/tls](https://github.com/nazmulidris/rust-scratch/tree/main/tls) - TLS
example using this protocol stack

## Generalized Architecture

Multiple "blocking resource threads" can coexist, each dedicated to different concerns:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Process Instance                                                           │
│                                                                            │
│  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐       │
│  │ Input Thread      │  │ mDNS Thread       │  │ IPC/Sync Thread   │       │
│  │ (stdin/SIGWINCH)  │  │ (peer discovery)  │  │ (state sync)      │       │
│  └─────────┬─────────┘  └─────────┬─────────┘  └─────────┬─────────┘       │
│            │                      │                      │                 │
│            ▼                      ▼                      ▼                 │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                      Async Coordination Layer                         │ │
│  │  • Aggregates events from all threads                                 │ │
│  │  • Maintains application state                                        │ │
│  │  • Routes commands to appropriate threads                             │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────┘
```

## Use Cases

### 1. Service Discovery (mDNS/Zeroconf)

```text
┌─────────────────────────────────┐              ┌──────────────────────────┐
│ mDNS Thread                     │   broadcast  │ Async Consumers          │
│ ─────────────────────────────── │ ──────────▶  │ ──────────────────────── │
│ • Owns UDP socket (224.0.0.251) │              │ • Discover peer instances│
│ • Blocks on multicast recv()    │              │ • Update peer list       │
│ • Sends service announcements   │ ◀──────────  │ • Request announcements  │
└─────────────────────────────────┘   commands   └──────────────────────────┘
```

**Feasibility**: Straightforward—UDP multicast is just another fd for mio to poll.

### 2. Clipboard/Scratch Area Sync

```text
┌─────────────────────────────────┐              ┌──────────────────────────┐
│ Clipboard Sync Thread           │   broadcast  │ Async Consumers          │
│ ─────────────────────────────── │ ──────────▶  │ ──────────────────────── │
│ • Owns connections to peers     │              │ • Update local clipboard │
│ • Blocks on socket events       │              │ • Display sync status    │
│ • Receives remote changes       │ ◀──────────  │ • Push local changes     │
└─────────────────────────────────┘   local clip └──────────────────────────┘
```

**Feasibility**: Requires peer discovery (mDNS) + sync protocol.

### 3. P2P Mesh State Synchronization

For shared state across multiple instances without central orchestration.

**Feasibility**: Use CRDTs (Conflict-free Replicated Data Types) for automatic conflict resolution.

---

## IPC Strategy: JSON over Sockets

A unified protocol for both same-host and cross-host communication.

### Same-Host IPC: Unix Domain Sockets

Unix sockets are ideal for multiple instances on the same machine:

| Advantage    | Explanation                                  |
| :----------- | :------------------------------------------- |
| **Fast**     | No network stack overhead—just memory copies |
| **Secure**   | File system permissions control access       |
| **Simple**   | Works like TCP but faster                    |
| **Reliable** | No packet loss, ordering guaranteed          |

#### Socket Path Strategy

```text
$XDG_RUNTIME_DIR/myapp/
├── coordinator.sock      # First instance becomes coordinator
├── peer-<pid-1>.sock     # Or: each instance gets unique socket
├── peer-<pid-2>.sock
└── peers.json            # Registry of active instances
```

**Option A: Coordinator Model**

```text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Instance 1   │     │ Instance 2   │     │ Instance 3   │
│ (coordinator)│◀───▶│   (peer)     │     │   (peer)     │
└──────┬───────┘     └──────────────┘     └──────────────┘
       │                    ▲                    ▲
       └────────────────────┴────────────────────┘
                    hub-and-spoke
```

- First instance creates `coordinator.sock` and becomes the hub
- Subsequent instances connect to the coordinator
- Coordinator broadcasts messages to all peers

**Option B: Peer-to-Peer Mesh**

```text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Instance 1   │◀───▶│ Instance 2   │◀───▶│ Instance 3   │
└──────┬───────┘     └──────────────┘     └──────┬───────┘
       │                                         │
       └─────────────────────────────────────────┘
                    full mesh
```

- Each instance creates its own socket (`peer-<pid>.sock`)
- Instances discover each other via directory listing or registry file
- Each instance connects to all others

### Cross-Host IPC: TCP Sockets

Same JSON protocol, different transport:

| Aspect        | Same-Host (Unix)                  | Cross-Host (TCP) |
| :------------ | :-------------------------------- | :--------------- |
| **Discovery** | Directory listing / registry file | mDNS/Zeroconf    |
| **Transport** | Unix domain socket                | TCP socket       |
| **Security**  | File permissions                  | TLS (optional)   |
| **Protocol**  | JSON (identical)                  | JSON (identical) |

### Protocol Reuse: The Key Insight

**The same JSON protocol works identically over Unix and TCP sockets.** This is a major
architectural advantage:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         UNIFIED JSON PROTOCOL                               │
│                                                                             │
│  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐  │
│  │ Same-Host IPC                   │   │ Cross-Host IPC                  │  │
│  │ ─────────────────────────────── │   │ ─────────────────────────────── │  │
│  │ Transport: Unix Domain Socket   │   │ Transport: TCP Socket           │  │
│  │ Framing:   Length-prefixed      │   │ Framing:   Length-prefixed      │  │
│  │ Payload:   JSON                 │   │ Payload:   JSON (identical!)    │  │
│  └─────────────────────────────────┘   └─────────────────────────────────┘  │
│                                                                             │
│  Same Message types, same serialization code, same business logic           │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Benefits:**

- **Code reuse**: Single serialization/deserialization implementation
- **Easy testing**: Test with Unix sockets locally, deploy with TCP
- **Transport agnostic**: Business logic doesn't care where messages come from
- **Gradual rollout**: Start with same-host, add cross-host later without protocol changes

### Crate Options

**We already have a self-contained implementation** in `tui/src/network_io/` that handles:

- Length-prefixed framing (`u64` prefix via `write_u64`/`read_u64`)
- Compression
- Bincode serialization
- Handshake + timeouts

**No external dependencies needed** for the framing layer. The existing `byte_io::try_write()` and
`byte_io::try_read()` already do what `LengthDelimitedCodec` does.

For reference, the Tokio ecosystem alternatives are:

| Crate           | Purpose                                                  | Needed?                    |
| :-------------- | :------------------------------------------------------- | :------------------------- |
| [`tokio-util`]  | `LengthDelimitedCodec` for length-prefixed framing       | **No** (we have `byte_io`) |
| [`tokio-serde`] | Serde integration with async streams (JSON feature flag) | Optional                   |
| [`serde_json`]  | JSON serialization                                       | If using JSON              |

[`tokio-util`]: https://docs.rs/tokio-util/latest/tokio_util/codec/length_delimited/
[`tokio-serde`]: https://docs.rs/tokio-serde

### Serialization Format: JSON vs Bincode

The existing `network_io` module uses **bincode** (binary). For P2P/IPC, consider the trade-offs:

| Aspect               | JSON                             | Bincode                         |
| :------------------- | :------------------------------- | :------------------------------ |
| **Size**             | Larger (text)                    | Smaller (binary)                |
| **Speed**            | Slower parsing                   | Faster parsing                  |
| **Debuggability**    | Human-readable, easy to debug    | Requires tools to inspect       |
| **Interop**          | Universal (any language)         | Rust-specific (mostly)          |
| **Schema evolution** | Flexible (ignore unknown fields) | Fragile (binary layout changes) |

**Recommendation:**

- **JSON** for P2P/IPC: Easier debugging, better interop with tools like `nc`, `socat`, `jq`
- **Bincode** for high-throughput: When performance matters more than debuggability

**Both use the same framing** (length-prefixed), so you can swap serialization formats without
changing the transport layer. The existing `network_io` module could be extended to support both:

```rust
// Conceptual - swap serializer without changing framing
enum Serializer {
    Json,     // Use serde_json
    Bincode,  // Use bincode (existing)
}
```

[`serde_json`]: https://docs.rs/serde_json

#### Example: Length-Prefixed JSON over Any AsyncRead/AsyncWrite

```rust
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::formats::Json;

/// Works with both UnixStream and TcpStream!
fn wrap_transport<T>(io: T) -> impl Stream<Item = Message> + Sink<Message>
where
    T: AsyncRead + AsyncWrite,
{
    // Layer 1: Length-prefixed framing (handles [u32 length][payload])
    let length_delimited = Framed::new(io, LengthDelimitedCodec::new());

    // Layer 2: JSON serialization on top of framed bytes
    tokio_serde::Framed::new(length_delimited, Json::<Message, Message>::default())
}

// Same function works for both:
let unix_transport = wrap_transport(unix_stream);  // Same-host IPC
let tcp_transport = wrap_transport(tcp_stream);    // Cross-host IPC
```

### Protocol Design: Length-Prefixed JSON

```text
┌─────────────┬──────────────────────────────────────┐
│ Length (4B) │ JSON Payload                         │
│ big-endian  │ UTF-8 encoded                        │
└─────────────┴──────────────────────────────────────┘
```

**Why length-prefixed over newline-delimited (NDJSON)?**

- Handles messages containing newlines
- Allows binary data (base64 encoded) without escaping issues
- Efficient parsing—know exactly how many bytes to read
- **Tokio ecosystem support**: `LengthDelimitedCodec` handles this out of the box

### Transport Abstraction (Blocking Thread Version)

For the dedicated blocking thread pattern (non-tokio), a simpler sync abstraction:

```rust
use std::io::{self, Read, Write};

/// Unified transport trait for Unix and TCP sockets (sync/blocking)
pub trait Transport: Send {
    fn send_message(&mut self, msg: &Message) -> io::Result<()>;
    fn recv_message(&mut self) -> io::Result<Message>;
}

impl<T: Read + Write + Send> Transport for T {
    fn send_message(&mut self, msg: &Message) -> io::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        let len = (payload.len() as u32).to_be_bytes();
        self.write_all(&len)?;
        self.write_all(&payload)?;
        self.flush()
    }

    fn recv_message(&mut self) -> io::Result<Message> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut payload = vec![0u8; len];
        self.read_exact(&mut payload)?;
        serde_json::from_slice(&payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
```

This works with `UnixStream`, `TcpStream`, or any `Read + Write` type.

### Message Types

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    // Peer management
    #[serde(rename = "hello")]
    Hello { instance_id: Uuid, capabilities: Vec<String> },

    #[serde(rename = "goodbye")]
    Goodbye { instance_id: Uuid },

    // Heartbeat
    #[serde(rename = "ping")]
    Ping { timestamp: u64 },

    #[serde(rename = "pong")]
    Pong { timestamp: u64 },

    // Clipboard sync
    #[serde(rename = "clipboard_update")]
    ClipboardUpdate {
        data: String,
        timestamp: u64,
        source_instance: Uuid,
    },

    // Generic state sync (CRDT operations)
    #[serde(rename = "state_delta")]
    StateDelta {
        crdt_ops: Vec<CrdtOperation>,
        vector_clock: HashMap<Uuid, u64>,
    },
}
```

Example JSON messages:

```json
{"type":"hello","instance_id":"550e8400-e29b-41d4-a716-446655440000","capabilities":["clipboard","state_sync"]}

{"type":"clipboard_update","data":"Hello, world!","timestamp":1702834567890,"source_instance":"550e8400-e29b-41d4-a716-446655440000"}

{"type":"ping","timestamp":1702834567890}
```

---

## P2P Mesh: Challenges & Solutions

| Challenge                       | Solution                                                 |
| :------------------------------ | :------------------------------------------------------- |
| **Peer discovery (same host)**  | Directory listing or registry file in `$XDG_RUNTIME_DIR` |
| **Peer discovery (cross host)** | mDNS/Zeroconf—zero config on LAN                         |
| **Conflict resolution**         | CRDTs or last-writer-wins with vector clocks             |
| **Network partitions**          | Eventual consistency—peers sync when reconnected         |
| **Ordering**                    | Lamport timestamps or vector clocks                      |
| **Membership changes**          | Gossip-based protocol or heartbeat timeout               |

### Why CRDTs?

CRDTs automatically resolve conflicts without coordination:

```text
Machine A: clipboard = "foo" (vector_clock: {A: 10})
Machine B: clipboard = "bar" (vector_clock: {B: 12})

After sync: both converge deterministically
No conflict! No central server needed!
```

For simple cases like clipboard, last-writer-wins with timestamps is sufficient. For complex state,
consider libraries like `yrs` (Yjs port) or `automerge`.

---

## Implementation Considerations

### Thread Count

Don't create too many blocking threads:

```text
✅ Good: 3-5 dedicated threads for distinct concerns
✅ Consider: Single mio thread managing related sockets (all network I/O)
❌ Avoid: One thread per peer connection (use mio to multiplex)
```

### Startup Ordering

Some threads depend on others:

```text
1. mDNS thread starts first → begins discovering peers
2. IPC/Sync thread starts → connects to discovered peers
3. Use LazyLock pattern for clean initialization
```

### Channel Topology

```text
Option A: Each thread has its own broadcast channel
          ✅ Simple, independent threads
          ✅ Current mio_poller approach

Option B: Central event bus
          ✅ Easier coordination across threads
          ⚠️ Single point of complexity
```

---

## Real-World Validation

This pattern powers production systems:

| System                  | How It Uses This Pattern                                       |
| :---------------------- | :------------------------------------------------------------- |
| **Syncthing**           | P2P file sync, discovery via local announce + global discovery |
| **Apple AirDrop**       | mDNS discovery + P2P transfer                                  |
| **Spotify Connect**     | mDNS discovery for local devices                               |
| **CRDTs in production** | Figma (multiplayer), Apple Notes, many collaborative apps      |

---

## Future Work

- [ ] Prototype mDNS discovery thread
- [ ] Prototype Unix socket IPC for same-host instances
- [ ] Design CRDT-based state synchronization
- [ ] Evaluate existing Rust crates: `mdns`, `zeroconf`, `yrs`, `automerge`
- [ ] Performance testing: message throughput, latency

---

## References

- [mio crate](https://docs.rs/mio) - Low-level I/O multiplexing
- [signal-hook-mio](https://docs.rs/signal-hook-mio) - Signal handling with mio
- [mdns crate](https://docs.rs/mdns) - mDNS/Zeroconf in Rust
- [CRDTs explained](https://crdt.tech/) - Conflict-free Replicated Data Types
- [Yrs (Yjs port)](https://docs.rs/yrs) - CRDT library for Rust
- [Automerge](https://automerge.org/) - CRDT library with Rust bindings
