#!/bin/sh

# one way to get latest version:
curl -SsI https://github.com/AustinWise/smeagol/releases/latest | grep '^location: ' | sed -e 's$.*/$$'
