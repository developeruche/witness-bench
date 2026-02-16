# Benchmark Report: Engine API Witness Generation

**Client Pair:** Geth (EL) / Lighthouse (CL)
**System**: M3 MAX, Macbook pro

**Code:** https://github.com/developeruche/witness-bench

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




## Ethrex Engine API Witness Generation
**Client Pair:** Ethrex (EL) / Lighthouse (CL)
**System:** M3 Pro, Macbook Pro

The benchmark compared the latency of the current decoupled flow (`newPayload` + `debug_executionWitness`) against the proposed coupled flow (`engine_newPayloadWithWitness`). 


> **Key Insight**: Unlike Geth, where the Coupled flow was a massive optimization, **Ethrex's Decoupled flow is already extremely fast (up to 4.7x faster than Geth)**. Consequently, the Coupled flow actually introduces a **regression** for large payloads, likely due to the overhead of hex-encoding the witness for the JSON-RPC transport. This should be resolved the transport layer is switch from TCP to unix domain sockets(IPC). There is still some optimization to be pressed out from the `serialization` though, but this only makes sense when the witness is very large, the more optimal optimation point is the transport layer.

**High-Level Findings:**

*   **Ethrex vs. Geth (Decoupled):** Ethrex is significantly faster. For a 500MB witness, Ethrex takes **4.8s** vs Geth's **22.6s** (**~4.7x faster**).
*   **Ethrex Coupled vs. Decoupled:**
    *   **Small Payloads (100MB):** Comparable performance (~1.1s).
    *   **Large Payloads (300MB+):** The Coupled flow is **slower**. For 300MB, Coupling adds ~3.6s of latency (2.1x slower).
*   **The Bottleneck:** The "Coupled" regression is driven by **Transport Overhead**. Hex-encoding the RLP witness for the Engine API likely doubles the payload size, saturating the transport layer.


### 1. Total Round Trip Time

*Comparison of the total time from the "Requestor" (CL/Client/ZK-Prover) initiating the request to receiving the full witness.*

#### The "Happy Path" (Block Depth = 0)

| Witness Size (Approx) | Ethrex Decoupled (ms) | Ethrex Coupled (ms) | Delta | Geth Decoupled (Ref) |
| :--- | :--- | :--- | :--- | :--- |
| **100 MB** | 1,160 ms | **1,117 ms** | **~Same** | 3,861 ms |
| **300 MB** | **3,176 ms** | 6,794 ms | **Coupled is 2.1x SLOWER** | 11,031 ms |
| **500 MB** | **4,848 ms** | 8,131 ms | **Coupled is 1.7x SLOWER** | 22,614 ms |

**Insight:** Ethrex's native(engine_newPayload +debug_executionWitness) witness generation is incredibly efficient (Decoupled). However, forcing this data through the Engine API (Coupled) adds significant overhead for large blocks. An Experiment would be done, same cases but different transport protocol, this should hopefully reduce the overhead greatly.


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


## IPC Transport layer

Before switching the transport layer used by the Engine API, a proof-of-concept (PoC) experiment was conducted to measure the potential speed improvements in data transfer that this change would introduce. The goal was also to establish a baseline throughput target for the client implementations (**Ethrex** and **Lighthouse**).

Below are the results for payload sizes of **100 MB**, **300 MB**, and **500 MB**:

| Payload Size | Wire Size | Round Trip Time (RTT) | Effective Throughput |
| :----------- | :-------- | :-------------------- | :------------------- |
| **100 MB**   | 100 MB    | 185 ms                | **566.8 MB/s**       |
| **300 MB**   | 300 MB    | 579 ms                | **543.3 MB/s**       |
| **500 MB**   | 500 MB    | 932 ms                | **562.5 MB/s**       |

code: https://github.com/developeruche/blockchain-protocol-experiment/tree/main/witness-transport

If we can achieve similar throughput using IPC (Unix domain sockets) instead of TCP, the coupled flow (`newPayloadWithWitness`) would be significantly faster than the uncoupled approach (`newPayload` + `debug_executionWitness`).

In that case, the round-trip time for payloads of **300 MB** and **500 MB** would be reduced to approximately **2 seconds** and **3 seconds**, respectively.


### Lighthouse + Ethrex [EngineAPI over IPC]
Switching to IPC yielded a measurable performance improvement over TCP, but did not fully replicate the raw throughput seen in the isolated IPC experiments.

*Comparison of Coupled Flow (TCP) vs. Coupled Flow (IPC).*

| Witness Size (Approx) | Ethrex Coupled (TCP) | **Ethrex Coupled (IPC)** | Improvement | Reference: Decoupled |
| --- | --- | --- | --- | --- |
| **100 MB** | 1,117 ms | **1,690 ms** | *Slower* | 1,160 ms |
| **300 MB** | 6,794 ms | **4,461 ms** | **~1.5x Faster** | 3,176 ms |
| **500 MB** | 8,131 ms | **6,003 ms** | **~1.35x Faster** | 4,848 ms |

Moving to IPC reduced the total latency for large payloads by approximately **26% to 34%**. While this is a significant step forward, the **Coupled IPC flow is still slower than the Decoupled flow** (which clocks in at ~4.8s for 500MB). The theoretical 2-3 second latency projected by the raw IPC experiment was not met in the integrated client environment.

I think why this is the case (I am not sure and my investigation have not answered the question just yet), the overhead might be coming from the `FileSystem Overhead`, In this test setup, the Unix socket resides on a mounted volume. This forces every data chunk to traverse VirtioFS (a high-overhead virtualization layer) to synchronize between the macOS host and the Linux VM. In contrast, the TCP connection utilizes Docker's internal network bridge, which stays entirely within the Linux VM's RAM, bypassing this costly filesystem boundary. On a bare-metal Linux machine (production environment), this filesystem bottleneck disappears. The socket file and kernel are local to the same OS, allowing IPC to function as a zero-copy memory transfer. Therefore, while TCP appears competitive in this specific virtualized context, IPC remains theoretically an order of magnitude faster for high-throughput local communication on native Linux servers. The results above should be considered a "worst-case scenario" for IPC. I couldn't provide data for this right now because Ethrex does not progress block unless there are a minimum of one extra client pair, once I figure this out, I should get some data backing this cliam.

**Latency Breakdown (Coupled Flow - IPC)**
*Isolating the new bottlenecks now that TCP is removed.*

**Scenario: 500MB Test (Block 218 - Actual Witness Size ~295 MB, this doubles before json transit)**

| Component | Duration (ms) | % of Total Time | Notes |
| --- | --- | --- | --- |
| **Block Production** | 70 ms | 1.2% | Consistent. |
| **Witness Generation** | 718 ms | 12% |  |
| **Serialization** | **1,278 ms** | **21%** | Remains a fixed cost. |
| **Transport / Overhead** | **3,937 ms** | **65.6%** | **Reduced, but still dominant.** |
| **TOTAL** | **6,003 ms** | **100%** |  |

The move to IPC confirms that the Transport Layer is the primary optimization target. While IPC provided a **~30% speedup** over TCP, the implementation has not yet saturated the available IPC bandwidth. 


## Side Line
This is a bit of an aside, but an important one. While trying to bloat the stateless witness to reach 100MB, 300MB, and 500MB, I ran into some challenges. My first approach involved spamming a block with contracts that did a lot of state writing (`SSTORE`). This didn't work well because I hit the gas limit long before the witness size even reached 100MB.
To fix this, I switched to making thousands of very cheap calls to different smart contracts using a Multicall contract. This was much more effective: I hit 100MB with just 8M gas, 300MB with 24M gas, and 500MB with 38M gas. If we extrapolate this, a 60M gas limit block could result in **750MB** of witness data, which displaces our initial assumption that 300MB was the worst-case scenario.
Crucially, as the state trie grows, it actually costs *less* gas to generate a large witness. Thankfully, this growth is logarithmic rather than linear or exponential.

Depending on how much time it cost for the final flow we choose, it might be wise to introduce some contract size metering similar to block gas limit to prevent this from becoming a problem or an attack plain for "[real time] prover killers" (increasing wait time for the prover/CL to recieve the witness before generating proofs can start).