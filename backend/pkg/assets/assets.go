package assets

import _ "embed"

//go:embed english.json
var Data []byte

//go:embed names.json
var Names []byte
