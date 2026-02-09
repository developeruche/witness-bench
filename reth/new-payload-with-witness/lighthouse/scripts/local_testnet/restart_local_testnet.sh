#!/usr/bin/env bash

# Requires `docker`, `kurtosis`, `yq`

set -Eeuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ENCLAVE_NAME=local-testnet
NETWORK_PARAMS_FILE=$SCRIPT_DIR/network_params.yaml
ETHEREUM_PKG_VERSION=5.0.1

BUILD_IMAGE=true
BUILDER_PROPOSALS=false
CI=false
KEEP_ENCLAVE=false
RUN_ASSERTOOR_TESTS=false

# Get options
while getopts "e:b:n:phcak" flag; do
  case "${flag}" in
    a) RUN_ASSERTOOR_TESTS=true;;
    e) ENCLAVE_NAME=${OPTARG};;
    b) BUILD_IMAGE=${OPTARG};;
    n) NETWORK_PARAMS_FILE=${OPTARG};;
    p) BUILDER_PROPOSALS=true;;
    c) CI=true;;
    k) KEEP_ENCLAVE=true;;
    h)
        echo "Start a local testnet with kurtosis."
        echo
        echo "usage: $0 <Options>"
        echo
        echo "Options:"
        echo "   -e: enclave name                                default: $ENCLAVE_NAME"
        echo "   -b: whether to build Lighthouse docker image    default: $BUILD_IMAGE"
        echo "   -n: kurtosis network params file path           default: $NETWORK_PARAMS_FILE"
        echo "   -p: enable builder proposals"
        echo "   -c: CI mode, run without other additional services like Grafana and Dora explorer"
        echo "   -a: run Assertoor tests"
        echo "   -k: keeping enclave to allow starting the testnet without destroying the existing one"
        echo "   -h: this help"
        exit
        ;;
  esac
done

LH_IMAGE_NAME=$(yq eval ".participants[0].cl_image" $NETWORK_PARAMS_FILE)
GETH_IMAGE_NAME=$(yq eval ".participants[0].el_image" $NETWORK_PARAMS_FILE)

GETH_ROOT_DIR="$SCRIPT_DIR/../../../go-ethereum"

kurtosis run --enclave $ENCLAVE_NAME github.com/ethpandaops/ethereum-package@$ETHEREUM_PKG_VERSION --args-file $NETWORK_PARAMS_FILE --image-download always

echo "Started!"
