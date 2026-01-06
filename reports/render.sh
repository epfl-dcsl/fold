#!/bin/sh

DIR=$(dirname $0)

for file in reports/**/report.typ; do
	echo Rendering $file...
    typst compile $file -f pdf --root $DIR/..
done
