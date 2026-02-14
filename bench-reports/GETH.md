# Benchmark Report: Engine API Witness Generation
**Client Pair:** Geth (EL) / Lighthouse (CL)
**System**: M3 pro, Macbook pro



The benchmark compared the latency of the current decoupled flow (`newPayload` + `debug_executionWitness`) against the proposed coupled flow (`engine_newPayloadWithWitness`). It is important to note that this report focuses on worst case scenarios, randomly sampling mainnet block the witness size is about "10mb to 15mb".

**Key Findings:**

* **Significant Latency Reduction:** The coupled flow consistently outperforms the decoupled flow across all witness sizes. The improvement ranges from **33%** (for 300MB payloads) to **71%** (for 100MB payloads).
* **Transport Bottleneck:** For the coupled flow, the largest latency component is the transport of the massive payload (network/RPC overhead), not the execution or serialization.
* **JSON Overhead:** While JSON serialization is measurable (e.g., ~2.1s for a 500MB witness), it is still drastically faster than the overhead of the decoupled approach.


### Total Round Trip Time

*Comparison of the total time from the "Requestor" (CL/Client/ZK-Prover) initiating the request to receiving the full witness. Ideally, this captures the "Apples-to-Apples" end-to-end latency.*

#### The "Happy Path" (Block Depth = 0)

*This measures the baseline efficiency when no rewind is required.*

| Witness Size (Approx) | Scenario A: Decoupled (Total ms) | Scenario B: Coupled (Total ms) | Improvement |
| --- | --- | --- | --- |
| **100 MB** | 3,861 ms | 1,118 ms | **3.4x Faster** |
| **300 MB** | 11,031 ms | 7,373 ms | **1.5x Faster** |
| **500 MB** | 22,614 ms | 7,237 ms | **3.1x Faster** |

> **Observation:** The coupled approach is significantly faster. Notably, the decoupled approach scales poorly with size (jumping to 22s for 500MB), whereas the coupled approach maintained stable performance between 300MB and 500MB (~7s).

The "Rewind Penalty" (Scenario A vs. B at Depth)

*Demonstrating the cost of the decoupled flow when the witness is requested for older blocks.*

| Block Depth | Witness Size | Scenario A: Decoupled (ms) | Scenario B: Coupled (ms) | Delta |
| --- | --- | --- | --- | --- |
| **8 Blocks** | 300 MB | 11,287 ms | 7,373 ms | **1.5x Faster** |
| **32 Blocks** | 300 MB | 11,404 ms | 7,373 ms | **1.5x Faster** |
| **64 Blocks** | 300 MB | *Not Supported* | 7,373 ms | **Infinite** |

> **Observation:** While the rewind penalty in Geth (Scenario A) adds only a marginal cost we would need to take this with a grain of salt as GETH node becomes stale after this request `debug_executionWitness`, the state management(to rewind to the present state) is dubuggy. 



### Component Breakdown (The "JSON Concern")

*Isolating the cost of the "JSON Penalty" vs. Execution vs. Transport in the Coupled Flow.*

### Latency Breakdown (Coupled Flow - 300MB Witness)

| Component | Duration (ms) | % of Total Time | Notes |
| --- | --- | --- | --- |
| **Block Execution** | 291 ms | 4% | Standard processing cost. |
| **Witness Generation** | 11 ms | <1% | Negligible. |
| **JSON Serialization** | **484 ms** | **6.5%** | **Minor overhead.** |
| **Transport / Overhead** | **6,587 ms** | **89%** | **The primary bottleneck.** |
| **TOTAL** | **7,373 ms** | **100%** |  |

### Latency Breakdown (Coupled Flow - 500MB Witness)

| Component | Duration (ms) | % of Total Time | Notes |
| --- | --- | --- | --- |
| **Block Execution** | 400 ms | 5.5% |  |
| **Witness Generation** | 25 ms | <1% |  |
| **JSON Serialization** | **2,175 ms** | **30%** | **Significant jump in serialization cost.** |
| **Transport / Overhead** | 4,637 ms | 64% |  |
| **TOTAL** | **7,237 ms** | **100%** |  |


> The fear that JSON serialization would cripple the performance is only partially founded. At 500MB, serialization takes ~2 seconds, which is significant. However, even with this cost, the **Total Coupled Latency (7.2s)** is still **3x faster** than the **Total Decoupled Latency (22.6s)**.



The data strongly supports moving to the **`engine_newPayloadWithWitness`** coupled flow.

1. **Performance:** It provides a 1.5x to 3x speedup in end-to-end latency.
2. **Stability:** It eliminates the complexity of rewinding state for deep blocks.
3. **Optimization Targets:** Future optimizations should focus on the **Transport Layer** (moving 300MB+ data efficiently) rather than optimizing Witness Generation, which is already negligible (~10-25ms).