# Studio templates

Data-driven verticals for Launch Studio. Each folder is one template:

```
<template-id>/
  template.json   # manifest (camelCase)
  program.lean    # ProgramV1 golden source
  abi.json        # optional solc ABI for Preview / interact demos
```

`preferredNetworkId` should stay on an X Layer preset (`xlayer-testnet` /
`xlayer-mainnet`) unless the vertical truly needs another EVM.

Design tokens for Preview HTML live in `_design/DESIGN.md` (Open Design–style
portable system; not a Vue runtime dependency).
