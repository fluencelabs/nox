## IMPORTANT: Client and README are under heavy development and can be outdated. Ask a question in gitter or file an issue.

## Fluence Dashboard

## Build

- Install project:

`npm install`

- Build production files:

`npm run build`

Then look at `build` directory

## Development

- Install project:

`npm install`

- Start dev server:

`npm run watch`

## Configuration

Address of Fluence contract can be configured in
- dashboard/src/constants.ts, see defaultContractAddress constant
- `data-contract` attribute of dashboard index.html body tag for example: `<body class="hold-transition skin-blue skin-fluence" id="root" data-contract="0xf008c29bb1fabd1eb1ea73b2410f523a5c213f19">`
- passed in querystring, `<dashboard url>/?contract=0xf008c29bb1fabd1eb1ea73b2410f523a5c213f19`