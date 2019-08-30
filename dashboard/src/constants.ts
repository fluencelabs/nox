/*
    Devnet
    Network contract: 0xeFF91455de6D4CF57C141bD8bF819E5f873c1A01
    Dashboard contract: 0x7392838158bA5E862BbDc679cD84949c8cFA4d09

    Stage
    Network contract: 0xe01690f60E08207Fa29F9ef98fA35e7fB7A12A96
    Dashboard contract: 0x18a6918103500b8517faf9C2A03377bF690407e0

    Dedicated
    Network contract: 0x78Da4Dd8315bB6A652d11F166bDb032EdffC4293
    Dashboard contract: 0x292DBdbE9383d6fcC345eb9B56818B1B0bF2D888
 */

export const defaultContractAddress: string = '0xe01690f60E08207Fa29F9ef98fA35e7fB7A12A96';
export const dashboardContractAddress: string = '0x18a6918103500b8517faf9C2A03377bF690407e0';

export const rootTagId: string = 'root';

export const fluenceNodeAddr = 'http://geth.fluence.one:8545';
export const fluenceIpfsAddr = 'http://ipfs.fluence.one:5001';

export const appUploadUrl = fluenceIpfsAddr + '/api/v0/add';
export const ipfsDownloadUrl = fluenceIpfsAddr + '/api/v0/cat?arg=';

export const account = "0xff8AD53f5776cCd8A0517bedc898f707E64c6159";
export const privateKey = Buffer.from("1EACC68D6AFF1F9E968001F4C94ACB92F586268E1C8CCA840AA23D6C55CDF375", 'hex');

export const llamaPrivateKey = "569ae4fed4b0485848d3cf9bbe3723f5783aadd0d5f6fd83e18b45ac22496859";
