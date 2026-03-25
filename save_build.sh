#!/bin/bash

cargo build --release

VERSION=$1
cp target/release/sabretooth releases/sabretooth_$VERSION

echo "Saved as sabretooth_$VERSION"