declare module 'supercop.js' {

    export = supercop;

    declare namespace supercop {
        function verify(signature: Buffer, message: Buffer, publicKey: Buffer): boolean;
    }

}
