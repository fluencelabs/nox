# Website with Fluence backend and static resources on IPFS

## Motivation

Fluence nodes are stateful but it is wasteful to store website (or something else) static resources on them. We can use decentralized storage like IPFS to store static resources and save the decentralized state of the whole system. 

## Requirements

- [IPFS CLI](https://docs.ipfs.io/introduction/install/) to upload resources

## How To

First of all, we need already written frontend interacting with Fluence nodes.

And then let's upload directory with built frontend code:
```
IPFS_ADDR=$(host data.fluence.one | awk '/has address/ { print $4 }')
ipfs --api /ip4/$IPFS_ADDR/tcp/5001 add -r path/to/built/frontend
```
`IPFS_ADDR` - is a data.fluence.one ip address

This command will return a hash of directory. We can check if the directory is uploaded by opening a link with this hash: `http://data.fluence.one:8080/ipfs/<IpfsHash>`

That's it! Now your frontend code is available with this link: `http://data.fluence.one:8080/ipfs/<IpfsHash>/index.html`

Note, that your entrypoint is not `index.html`, change the link to right one. 