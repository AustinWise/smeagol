#!/bin/sh

set -ex

cd $(dirname "$0")

npm install
cp node_modules/@primer/css/dist/primer.css ../static/
cp node_modules/@primer/css/dist/primer.css.map ../static/