# idiomatic-sh-ryo-derived

Owns portable derived-view meaning through the public inbound prepare_derived, build and view use cases. The execution engine owns acquisition, retention, scheduling, publication, cancellation, and lifetime. Domain implementation modules stay behind the use cases.

## Domains

### adjacency

Builds the compact adjacency of engine-admitted edge rows, partitioned by key, with validated single-parent ancestry.

### allocation

Names the numbers an allocation is described by: its identity, its element capacity and its bytes.

### closure_adjacency

Builds one partition's closure adjacency from its edge rows, reusing a recycled partition's buffers.

### contract

Declares the finite build inputs, the borrowed views and the sealed prepared requests and payloads the use cases exchange.

### cycle_adjacency

Numbers the nodes of engine-admitted edge rows in first-seen order and groups each edge under its source.

### dfs

Prepares roots and ordered edges, with canonical columns, indexes and per-edge runs, for the engine's depth-first traversal.

### dispatch

Dispatches a prepared request and its inputs to the one build of its kind, and a payload to the one view of its kind.

### equality

Indexes the keys of finite rows in hash chains, with buffers a later build reuses.

### fault

Declares the portable failures of derived semantics.

### footprint

Reports the allocation facts of a payload and of a build's scratch; the engine alone assigns lifetimes and budgets.

### key

Reduces cells to words and keys to hashes, shared by construction and by immutable physical probes.

### membership

Collects the members of a column and marks them as values or over a dense element space.

### native

Reduces a static join shape over native tables to the lookup it stands for, or declines.

### ordered

Pairs two ordered sides by key, keeping owned keys and refusing duplicate orders.

### partition

Orders rows by typed key, keeping a clustered stream as it came and sorting returning key runs.

### positions

Names row positions and membership marks of the finite derived row sets.

### predicate

Lists the rows a predicate accepts, the first row of each distinct key and the postings of literal texts.

### preparation

Checks a request against the schemas of its inputs without reading rows, and keeps the verdict.

### raw_order

Orders the rows of a flat list of lowered key words, keeping duplicates in their original order.

### raw_words

Names the lowered key words, strides and places of raw key ordering.

### request

Declares the static requirements of every derived kind, semantics only.

### row_cells

Declares the finite row views and the text services a derived build reads through.

### sharing

Shares immutable positions, marks, key indexes and graphs whose identity never depends on liveness.

### vertices

Names the partitions, vertices and edge offsets of a compact adjacency.
