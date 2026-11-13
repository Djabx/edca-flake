# edcas-common

Shared types for the [EDCAS](https://edcas.de) workspace — *Elite Dangerous
Commander Assistant System*. This crate has no binary and no runtime logic of its
own; it is the common vocabulary that the client, server and web app all speak so
their wire formats can never drift apart.

It is consumed as a path dependency by [`edcas-client`](../client),
[`edcas-server`](../server) and [`edcas-web`](../web).

## What's in it

- **`api`** — the request and response types for the edcas REST API
  (`SystemResponse`, `BodyResponse`, `StationResponse`, `FactionResponse`,
  trade-route/commodity types, `DatabaseStatsResponse`, `ServerTickResponse`,
  `ClientVersionResponse`, the search/query parameter structs, …). The server
  serialises them, the client and web app deserialise them — one definition, both
  ends.
- **`journal`** — strongly-typed models for the Elite Dangerous journal / EDDN
  message bodies the pipeline cares about, grouped by domain:
  `travel` (FSDJump, Location, CarrierJump), `scan`, `station` (Docked, market /
  outfitting / shipyard), `colonisation`, and shared `types`. `JournalEvent`
  parses a raw EDDN-format message into the right typed variant.

## Why a separate crate

The journal types are needed by both the **server** (to ingest EDDN + uploaded
events) and the **client** (to read the player's journal and build EDDN uploads),
and the API types are needed by everything that talks to the REST API. Keeping
them here means a schema change is made once and the compiler enforces it across
all three consumers.

Everything is `serde`-(de)serialisable; many types also derive `PartialEq`/`Clone`
so they can be used directly as Dioxus component props in `edcas-web`.
