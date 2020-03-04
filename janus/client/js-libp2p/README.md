# Janus browser client
Browser client for the Fluence network based on the js-libp2p.

## How to build

With `npm` installed building could be done as follows:

```bash
npm install

npm run build
```

## How to run

This command will run webpack developer server and launch browser with index.html

```bash
npm run start
```

Then in browser type a multiaddr of a server and press the `Connect` button.

Example of such multiaddr: `/ip4/127.0.0.1/tcp/9999/ws/p2p/QmU3LRNpe2R6VrRThhc2aR7Z4FS4AU38iHNEUAieNdRHVo`

After that you could see incoming messages and send them through the `Message` input.

Message should be given in the follows format: `{"dst": "QmbCSdMTuYaPxVN49WNGGbyagxCAwZNYHommHN5g8f3vhP", "message": "11"}`
