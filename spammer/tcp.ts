import * as net from 'net';

const MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_NUMBER = 0x01;

const HEADER_SIZE = 9; // 1 byte type + 8 bytes length
const MAX_PAYLOAD_SIZE = 5n * 1024n * 1024n * 1024n; // 5 GB


function packHeader(msgType: number, length: bigint): Buffer {
    const buf = Buffer.alloc(HEADER_SIZE);
    buf.writeUInt8(msgType, 0);
    buf.writeBigUInt64BE(length, 1);
    return buf;
}

function unpackHeader(buf: Buffer): { msgType: number; payloadLen: bigint } {
    const msgType = buf.readUInt8(0);
    const payloadLen = buf.readBigUInt64BE(1);
    return { msgType, payloadLen };
}

import { TCP_HOST, TCP_PORT } from './constants';

export async function get_witness_by_block_number(block_number: number): Promise<Buffer> {
    return new Promise((resolve, reject) => {
        const client = new net.Socket();
        let headerBuf = Buffer.alloc(0);
        let payloadLen: bigint | null = null;
        let msgType: number | null = null;
        let totalRead = 0n;
        const chunks: Buffer[] = [];

        client.connect(TCP_PORT, TCP_HOST, () => {
            const reqPayload = Buffer.alloc(8);
            reqPayload.writeBigUInt64BE(BigInt(block_number), 0);

            const reqHeader = packHeader(MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_NUMBER, BigInt(reqPayload.length));
            client.write(reqHeader);
            client.write(reqPayload);
        });

        client.on('data', (rawChunk) => {
            const chunk = rawChunk as Buffer;

            if (payloadLen === null) {
                headerBuf = Buffer.concat([headerBuf, chunk]);
                if (headerBuf.length >= HEADER_SIZE) {
                    const header = headerBuf.subarray(0, HEADER_SIZE);
                    const unpacked = unpackHeader(header);
                    msgType = unpacked.msgType;
                    payloadLen = unpacked.payloadLen;

                    if (msgType !== MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_NUMBER) {
                        client.destroy(new Error(`Unexpected response message type: ${msgType}`));
                        return;
                    }

                    if (payloadLen > MAX_PAYLOAD_SIZE) {
                        client.destroy(new Error(`Server responded with payload length ${payloadLen} exceeding MAX_PAYLOAD_SIZE`));
                        return;
                    }

                    if (payloadLen === 0n) {
                        client.destroy();
                        resolve(Buffer.alloc(0));
                        return;
                    }

                    const remainingChunk = headerBuf.subarray(HEADER_SIZE);
                    if (remainingChunk.length > 0) {
                        chunks.push(remainingChunk);
                        totalRead += BigInt(remainingChunk.length);
                    }
                }
            } else {
                chunks.push(chunk);
                totalRead += BigInt(chunk.length);
            }

            if (payloadLen !== null && totalRead >= payloadLen) {
                client.destroy();
                resolve(Buffer.concat(chunks));
            }
        });

        client.on('error', (err: Error) => {
            client.destroy();
            reject(err);
        });

        client.on('close', () => {
            if (payloadLen !== null && totalRead < payloadLen) {
                reject(new Error(`Connection closed before full payload was received. Read ${totalRead} of ${payloadLen} bytes`));
            }
        });
    });
}