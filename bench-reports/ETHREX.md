# Benchmark Report: Ethrex Engine API Witness Generation
**Client Pair:** Ethrex (EL) / Lighthouse (CL)
**System:** M3 Pro, Macbook Pro

The benchmark compared the latency of the current decoupled flow (`newPayload` + `debug_executionWitness`) against the proposed coupled flow (`engine_newPayloadWithWitness`). 


> **Key Insight**: Unlike Geth, where the Coupled flow was a massive optimization, **Ethrex's Decoupled flow is already extremely fast (up to 4.7x faster than Geth)**. Consequently, the Coupled flow actually introduces a **regression** for large payloads, likely due to the overhead of hex-encoding the witness for the JSON-RPC transport. This should be resolved the transport layer is switch from TCP to unix domain sockets(IPC). There is still some optimization to be pressed out from the `serialization` though, but this only makes sense when the witness is very large, the more optimal optimation point is the transport layer.

**High-Level Findings:**

*   **Ethrex vs. Geth (Decoupled):** Ethrex is significantly faster. For a 500MB witness, Ethrex takes **4.8s** vs Geth's **22.6s** (**~4.7x faster**).
*   **Ethrex Coupled vs. Decoupled:**
    *   **Small Payloads (100MB):** Comparable performance (~1.1s).
    *   **Large Payloads (300MB+):** The Coupled flow is **slower**. For 300MB, Coupling adds ~3.6s of latency (2.1x slower).
*   **The Bottleneck:** The "Coupled" regression is driven by **Transport Overhead**. Hex-encoding the RLP witness for the Engine API doubles the payload size, saturating the transport layer.


### 1. Total Round Trip Time

*Comparison of the total time from the "Requestor" (CL/Client/ZK-Prover) initiating the request to receiving the full witness.*

#### The "Happy Path" (Block Depth = 0)

| Witness Size (Approx) | Ethrex Decoupled (ms) | Ethrex Coupled (ms) | Delta | Geth Decoupled (Ref) |
| :--- | :--- | :--- | :--- | :--- |
| **100 MB** | 1,160 ms | **1,117 ms** | **~Same** | 3,861 ms |
| **300 MB** | **3,176 ms** | 6,794 ms | **Coupled is 2.1x SLOWER** | 11,031 ms |
| **500 MB** | **4,848 ms** | 8,131 ms | **Coupled is 1.7x SLOWER** | 22,614 ms |

**Insight:** Ethrex's native(engine_newPayload + debug_executionWitness) witness generation is incredibly efficient (Decoupled). However, forcing this data through the Engine API (Coupled) adds significant overhead for large blocks. An Experiment would be done, same cases but different transport protocol, this should hopefully reduce the overhead greatly.


### 2. Component Breakdown (Coupled Flow)

*Isolating where the time is spent in the Coupled Flow to identify the regression source.*

#### Latency Breakdown (300MB Witness - Block 192)

| Component | Duration (ms) | % of Total Time | Notes |
| :--- | :--- | :--- | :--- |
| **Block Production** | 34 ms | 0.5% | Extremely fast. |
| **Witness Generation** | 277 ms | 4% | Very fast (~300ms vs Geth's ~10-25ms). |
| **Serialization (RLP+Hex)** | **1,308 ms** | **19%** | Significant cost to encode. |
| **Transport / Overhead** | **5,175 ms** | **76%** | **The Bottleneck.** |
| **TOTAL** | **6,794 ms** | **100%** | |

#### Latency Breakdown (500MB Witness - Block 210)

| Component | Duration (ms) | % of Total Time | Notes |
| :--- | :--- | :--- | :--- |
| **Block Production** | 60 ms | 0.7% | |
| **Witness Generation** | 528 ms | 6.5% | |
| **Serialization (RLP+Hex)** | **1,499 ms** | **18%** | |
| **Transport / Overhead** | **6,044 ms** | **74%** | **The Bottleneck.** |
| **TOTAL** | **8,131 ms** | **100%** | |

**Insights:** 
1.  **Serialization**: Taking ~1.5s to serialize is significant, but not the main issue.
2.  **Transport**: The ~5-6s transport time is massive. In the Decoupled flow for the same size, the total time (Generation + Transport) was only ~3.2s - 4.8s. The Hex encoding required for the Engine API JSON payload doubles the data size (1 byte -> 2 hex chars), which explains why the transport cost explodes compared to the raw binary stream tailored for the Decoupled endpoint.



**Ethrex is Highly Performant:** Its base witness generation and decoupled retrieval are exceptional, outperforming Geth by a wide margin (3-5x).
**Coupled Flow Needs Binary Transport:** The current JSON-RPC Engine API specification requires hex encoding, which cripples performance for large witness payloads.
**Compression:** An experiment could be done to see if compressing the witness before sending it to the CL would help, but this would only be considered is the result of the transport layer change is not enough to fix the issue (which is quite unlikely... ).

**Next Steps I would be taking:** For **Ethrex**, the Decoupled flow is currently superior for large blocks. To make the Coupled flow viable for 300MB+ witnesses, we would first focus on transporting JSON over IPC (unix domain sockets) instead of TCP, then we would look into transporting this data in it RLP(very close to binary format), this might not be need though because the report so far shows that the `serialization` is cheaper than the `transport`, which may not be the case when transporting over IPC. 