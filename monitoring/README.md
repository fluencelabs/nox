## IMPORTANT: Client and README are under heavy development and can be outdated. Ask a question in gitter or file an issue.

## Fluence Monitoring

**Fluence Monitoring** collects all status data from Fluence contract and from all registered nodes and returns it as JSON.

## Goal

To give a possibility to observe whole Fluence network parts (contract, registered nodes, workers, validators, etc.) from one place.

## Usage

- Install project:

`npm install`

- Run server:

`npm run start:dev`

- Open browser on `localhost`
- Check if MetaMask is installed and correct ethereum network is selected (or local node is started)
- Check an address of Fluence contract (by default you can use `0x9995882876ae612bfd829498ccd73dd962ec950a` if you use `ganache` from our repo)
- Write in browser console: `showStatus(FLUENCE_CONTRACT_ADDRESS)`
- Plain collapsed JSON with all info about network will appear on the page

To be able to use monitoring on remote machines, repeat all steps on a remote machine and run `npm run start:prod`.