const fs = require('fs');
let octicons = require("@primer/octicons");

fs.writeFileSync(__dirname + '/../static/file.svg', octicons.file.toSVG({'aria-label': 'File'}));
fs.writeFileSync(__dirname + '/../static/file_directory.svg', octicons["file-directory-fill"].toSVG({'aria-label': 'Directory'}));
