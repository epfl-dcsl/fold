#!/bin/bash
if ! command -v readelf &> /dev/null; then
	echo "Missing readelf in PATH"
	exit 1
fi
if ! command -v bc &> /dev/null; then
	echo "Missing bc in PATH"
	exit 1
fi
if ! command -v gnuplot &> /dev/null; then
	echo "Missing gnuplot in PATH"
	exit 1
fi

average() {
	count=0
	sum=0

	for val in "$@"; do
		count=$((count + 1))
		sum=$((sum + $val))
	done

	bc <<< "scale=4;$sum / $count"
}

get-segment-field() {
	readelf -lW $1 2> /dev/null | grep TLS | xargs echo | cut -d ' ' -f $2
}

DAT=${DAT:-plot.data}
echo '#fsize msize' > $DAT

for file in $(find -L /lib -type f -name '*.so'); do
	fs=$(($(get-segment-field $file 5)))
	ms=$(($(get-segment-field $file 6)))

	fs=${fs:-0}
	ms=${ms:-0}

	file_sizes="$file_sizes $fs"
	mem_sizes="$mem_sizes $ms"

	echo $fs $ms >> $DAT
done

echo "Average TLS file size: $(average $file_sizes)"
echo "Average TLS mem size: $(average $mem_sizes)"

gnuplot -p -e "
set title 'TLS memory footprint';
set xlabel 'File size';
set ylabel 'Memory size';
plot '$DAT' using 1:2 with points pt 7 ps 0.5 title '';
"
