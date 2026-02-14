set -Eeuo pipefail
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
LH_IMAGE_NAME="lighthouse:local"


echo "Building Lighthouse Docker image."
ROOT_DIR="$SCRIPT_DIR/../baseline/lighthouse"
docker build --build-arg FEATURES=portable,spec-minimal -f $ROOT_DIR/Dockerfile -t $LH_IMAGE_NAME $ROOT_DIR


cd ../baseline/ethrex
make localnet