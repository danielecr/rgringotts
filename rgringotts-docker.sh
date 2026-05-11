#!/bin/bash
# rgringotts-docker.sh — run rgringotts in Docker with local data

create_folders() {
## Create 2 fake data folders for testing in this current directory:
    mkdir -p testdata/main
    mkdir -p testdata/archive
}
run_container() {
docker run -d \
  --name rgringotts \
  -p 127.0.0.1:7979:7979 \
  -v "$PWD/testdata/main:/data/main:rw" \
  -v "$PWD/testdata/archive:/data/archive:rw" \
  rgringotts \
  rgringotts --host 0.0.0.0 --port 7979 \
             --folder main=/data/main \
             --folder archive=/data/archive
}

## provide cmds: run, stop, logs, clean
case "${1:-}" in
    run)
        create_folders
        run_container
        ;;
    stop)
        docker stop rgringotts
        ;;
    logs)
        docker logs -f rgringotts
        ;;
    rmdocker)
        docker rm -f rgringotts
        ;;
    clean)
        docker rm -f rgringotts
        rm -rf testdata
        ;;
    *)
        echo "Usage: $0 {run|stop|logs|clean}"
        exit 1
        ;;
esac