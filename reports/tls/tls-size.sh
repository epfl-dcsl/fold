#!/bin/bash
if ! command -v readelf &> /dev/null; then
	echo "Missing readelf in PATH"
	exit 1
fi

get-segment-field() {
	readelf -lW $1 2> /dev/null | grep TLS | xargs echo | cut -d ' ' -f $2
}

for file in $(find / -type f -name '*.so'); do
	fs=$(($(get-segment-field $file 5)))
	ms=$(($(get-segment-field $file 6)))

	fs=${fs:-0}
	ms=${ms:-0}

	echo $file,$fs,$ms
done
