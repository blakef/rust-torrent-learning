# Rust Learning Schedule:

Week 1: S1 - S3
Week 2: S4 - S6

## Sub Project:

Single-file torrent, one HTTP tracker, download-only, verify against piece hashes, write to disk.

### What I'm implementing

Four protocol pieces, roughly in dependency order:

**Bencode**, the encoding everything else is written in. Four types:
1. integers i42e, 
2. length-prefixed byte strings 4:spam,
3. lists l…e, and 
4. dicts d…e (keys are byte strings, sorted).

A few hundred lines. This is what you make zero-copy in the Lifetimes week — the decoder borrows &[u8] slices out of the input rather than allocating.

**.torrent file (metainfo), a bencoded dict**: top level has announce (tracker URL) and info (a dict with name, piece length, length, and pieces — the last being 20-byte SHA-1 hashes concatenated into one long byte string, one per piece).
The gotcha that trips everyone: the infohash is the SHA-1 of the raw bencoded bytes of the info dict, exactly as they appear in the file — not a re-encoding of your parsed version. If you re-serialise, key ordering or integer formatting can differ and the hash won't match any peer. This is the concrete reason the zero-copy parser matters: you keep a slice pointing at the original info-dict bytes and hash those. It's the cleanest possible motivation for the Lifetimes chapter.

**Tracker (HTTP)**: a GET to the announce URL with query params: info_hash and peer_id (both raw 20-byte values, percent-encoded — not hex), plus port, uploaded, downloaded, left, and compact=1. The response is a bencoded dict with an interval and a peers blob. With compact=1 (BEP 23) each peer is 6 bytes: 4-byte IPv4 + 2-byte big-endian port. You fire the GET as raw HTTP over a TcpStream to stay std-only.

**Peer wire protocol, a TCP conversation**: first a fixed 68-byte handshake: one length byte (19), the string BitTorrent protocol, 8 reserved bytes, the 20-byte infohash, and your 20-byte peer_id. Verify the infohash the peer sends back matches. After that, length-prefixed messages: a 4-byte big-endian length, a 1-byte id, then payload. The ids you need:
* 0 choke / 1 unchoke — whether the peer will serve you
* 2 interested / 3 not interested — what you tell them
* 4 have / 5 bitfield — which pieces the peer holds
* 6 request / 7 piece — ask for a block, receive a block
* (length-0 keep-alives have no id; 8 cancel and 9 port you can ignore)

**Download loop**: connect → handshake → peer sends bitfield → you send interested → wait for unchoke → request blocks (16 KB each — pieces are much bigger, so a 256 KB piece is 16 requests) → collect the piece responses → once a full piece is assembled, SHA-1 it and compare to the hash from pieces. Match → write to disk at offset index × piece_length, mark done. Mismatch → discard and re-request. Repeat until every piece verifies.

### Architecture

```
src/
  main.rs        args, wire it together, box errors at the top
  bencode.rs     zero-copy decoder
  metainfo.rs    .torrent -> Torrent struct + infohash
  tracker.rs     announce request/response; Tracker trait
  peer/
    mod.rs       connection: handshake + message loop
    message.rs   PeerMessage enum + encode/decode (the macro lives here)
  piece.rs       piece picker, block assembly, hash verification
  storage.rs     write verified pieces to the file
  state.rs       shared TorrentState
  error.rs       TorrentError enum
```


## Schedule

### 🔁 Refresher Notes — go fast (completed ~2 months ago):
- [ ] [S1/A] Hello World
- [ ] [S1/A] Primitives
- [ ] [S1/A] Custom Types
- [ ] [S1/A] Variable Bindings
- [ ] [S1/A] Types
- [ ] [S1/A] Conversion
- [ ] [S2/B] Expressions
- [ ] [S2/B] Flow of Control
- [ ] [S2/B] Functions
- [ ] [S2/B] Modules
- [ ] [S2/B] Crates
- [ ] [S2/B] Cargo
- [ ] [S2/B] Attributes

### 🌱 New ground — full depth:
- [ ] Generics ← you stopped here; start of the real work.  Starting to implement the Simple Torrent Client from this point.
    - [ ] [S3] type params, bounds, multiple bounds, where-clauses
        - [ ] [T] define the domain types. PeerMessage enum, Torrent/Info structs, a generic Bitfield/piece store. Hardcode a parsed torrent or use test bytes so you have something to type against.
    - [ ] [S4/W2] associated types, phantom types (closes Generics)
        - [ ] [T] tracker-response types, and a piece-iterator scaffold ("pieces I still need").
- [ ] Scoping rules
    - [ ] [S5/W2] RAII, moves/ownership, borrowing, aliasing, ref
        - [ ] [T] decide the ownership model — who owns the input buffer, who owns shared state. A first, possibly owned bencode decoder to get moving.
    - [ ] [S6/W2] the full chapter, fresh start (annotations, fns, methods, structs, bounds, coercion, elision)
        - [ ] [T] rewrite bencode as zero-copy (borrowing slices), parse the metainfo, and compute the infohash from the raw info-dict slice. The chapter's whole lesson, applied.
- [ ] Traits
    - [ ] [S7/W3] derive, operator overloading, Drop, Iterator, impl Trait
        - [ ] [T] a Tracker trait, Display for a status line, and the HTTP GET over TcpStream with compact-peer parsing.
    - [ ] [S8/W3] dyn/returning traits, supertraits, disambiguation
        - [ ] [T] the piece picker as an Iterator; finish the tracker path end to end (URL → list of peer addresses).
- [ ] [S9/W3] Macros
    - [ ] [T] a message!-style macro declaring each peer-message id and its payload layout once, generating the encode/decode arms.
- [ ] Error handling
    - [ ] [S10/W4] panic, Option/unwrap, ? on Option, map/and_then
        - [ ] [T] introduce TorrentError; wrap bencode + metainfo parsing in Result; the handshake.
    - [ ] [S11/W4] Result, ?, early returns, combinators
        - [ ] [T] ? through the peer message loop; handshake infohash verification; connection/timeout errors.
    - [ ] [S12/W4] multiple error types, boxing, iterating over Results (+ buffer)
        - [ ] [T] piece-hash-mismatch handling, dropped-peer recovery, and boxing errors at main. The ⚠️ week paying off on real failure modes.
- [ ] [S13/W5] Std library types
    - [ ] [T] the shared state — HashMap<PeerId, PeerState>, Arc<Mutex<PiecePicker>>, bitset availability.
- [ ] [S14/W5] Std misc — threads, channels, paths, file I/O, child processes
    - [ ] [T] the spine — spawn peer threads, channels to the assembler, the wire protocol over TcpStream, write verified pieces at their offsets. First real download.
- [ ] [S15/W5] Std misc finish + Testing — unit, integration, doc tests
    - [ ] [T] close the loop; then bencode round-trip tests, handshake byte-layout tests, and full SHA-1 verification against a small known torrent (reproducible, no live swarm needed).

### 🔖 Revisit:
- [ ] [S16/W6] Unsafe Operations + Compatibility + Meta
    - [ ] [T] decoupled, as flagged — read unsafe/FFI and do a small standalone exercise, then doc-comment and benchmark the download loop.
- [ ] [S17/W6]  Buffer / revisit Lifetimes + Error handling (the two you're most likely to want a second pass on)
    - [ ] [T] polish, or a second pass on Lifetimes and Error handling.
