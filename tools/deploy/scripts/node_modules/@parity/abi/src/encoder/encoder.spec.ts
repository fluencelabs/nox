// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import Encoder from './encoder';
import Token from '../token';
import { padAddress, padFixedBytes, padU32 } from '../util/pad';

describe('encoder/Encoder', () => {
  describe('encodeToken', () => {
    it('requires token as Token', () => {
      expect(() => Encoder.encodeToken(undefined as any)).toThrow(/Token/);
    });

    it('encodes address tokens in Mediate(raw)', () => {
      const mediate = Encoder.encodeToken(new Token('address', '123'));

      expect(mediate.type).toEqual('raw');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes bool tokens in Mediate(raw)', () => {
      const mediatet = Encoder.encodeToken(new Token('bool', true));
      const mediatef = Encoder.encodeToken(new Token('bool', false));

      expect(mediatet.type).toEqual('raw');
      expect(mediatet.value).toBeTruthy();

      expect(mediatef.type).toEqual('raw');
      expect(mediatef.value).toBeTruthy();
    });

    it('encodes int tokens in Mediate(raw)', () => {
      const mediate = Encoder.encodeToken(new Token('int', '123'));

      expect(mediate.type).toEqual('raw');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes uint tokens in Mediate(raw)', () => {
      const mediate = Encoder.encodeToken(new Token('uint', '123'));

      expect(mediate.type).toEqual('raw');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes fixedBytes tokens in Mediate(raw)', () => {
      const mediate = Encoder.encodeToken(new Token('fixedBytes', '123'));

      expect(mediate.type).toEqual('raw');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes bytes tokens in Mediate(prefixed)', () => {
      const mediate = Encoder.encodeToken(new Token('bytes', '123'));

      expect(mediate.type).toEqual('prefixed');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes string tokens in Mediate(prefixed)', () => {
      const mediate = Encoder.encodeToken(new Token('string', '123'));

      expect(mediate.type).toEqual('prefixed');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes fixedArray tokens in Mediate(fixedArray)', () => {
      const mediate = Encoder.encodeToken(
        new Token('fixedArray', [new Token('uint', '123')])
      );

      expect(mediate.type).toEqual('fixedArray');
      expect(mediate.value).toBeTruthy();
    });

    it('encodes array tokens in Mediate(array)', () => {
      const mediate = Encoder.encodeToken(
        new Token('array', [new Token('uint', '123')])
      );

      expect(mediate.type).toEqual('array');
      expect(mediate.value).toBeTruthy();
    });

    it('throws an Error on invalid tokens', () => {
      const token = new Token('address');

      // @ts-ignore We uglily set the Token type here.
      token._type = 'noMatch';

      expect(() => Encoder.encodeToken(token)).toThrow(/noMatch/);
    });

    it('throws and error on invalid value tokens', () => {
      const token = new Token('uint');

      // @ts-ignore We uglily set the Token value here.
      token._value = 'invalidNumber';

      expect(() => Encoder.encodeToken(token)).toThrow(/Cannot encode/);
    });
  });

  describe('encode', () => {
    it('requires tokens array', () => {
      expect(() => Encoder.encode(undefined as any)).toThrow(/array/);
    });

    describe('addresses', () => {
      const address1 = '1111111111111111111111111111111111111111';
      const address2 = '2222222222222222222222222222222222222222';
      const address3 = '3333333333333333333333333333333333333333';
      const address4 = '4444444444444444444444444444444444444444';
      const encAddress1 = padAddress(address1);
      const encAddress2 = padAddress(address2);
      const encAddress3 = padAddress(address3);
      const encAddress4 = padAddress(address4);
      const tokenAddress1 = new Token('address', address1);
      const tokenAddress2 = new Token('address', address2);
      const tokenAddress3 = new Token('address', address3);
      const tokenAddress4 = new Token('address', address4);

      it('encodes an address', () => {
        const token = tokenAddress1;

        expect(Encoder.encode([token])).toEqual(encAddress1);
      });

      it('encodes an array of addresses', () => {
        const expected = `${padU32(0x20)}${padU32(
          2
        )}${encAddress1}${encAddress2}`;
        const token = new Token('array', [tokenAddress1, tokenAddress2]);

        expect(Encoder.encode([token])).toEqual(expected);
      });

      it('encodes an fixedArray of addresses', () => {
        const expected = `${encAddress1}${encAddress2}`;
        const token = new Token('fixedArray', [tokenAddress1, tokenAddress2]);

        expect(Encoder.encode([token])).toEqual(expected);
      });

      it('encodes two addresses', () => {
        const expected = `${encAddress1}${encAddress2}`;
        const tokens = [tokenAddress1, tokenAddress2];

        expect(Encoder.encode(tokens)).toEqual(expected);
      });

      it('encodes fixed array of dynamic array addresses', () => {
        const tokens1 = new Token('array', [tokenAddress1, tokenAddress2]);
        const tokens2 = new Token('array', [tokenAddress3, tokenAddress4]);
        const fixed = new Token('fixedArray', [tokens1, tokens2]);
        const expected = `${padU32(0x40)}${padU32(0xa0)}${padU32(
          2
        )}${encAddress1}${encAddress2}${padU32(2)}${encAddress3}${encAddress4}`;

        expect(Encoder.encode([fixed])).toEqual(expected);
      });

      it('encodes dynamic array of fixed array addresses', () => {
        const tokens1 = new Token('fixedArray', [tokenAddress1, tokenAddress2]);
        const tokens2 = new Token('fixedArray', [tokenAddress3, tokenAddress4]);
        const dynamic = new Token('array', [tokens1, tokens2]);
        const expected = `${padU32(0x20)}${padU32(
          2
        )}${encAddress1}${encAddress2}${encAddress3}${encAddress4}`;

        expect(Encoder.encode([dynamic])).toEqual(expected);
      });

      it('encodes dynamic array of dynamic array addresses', () => {
        const tokens1 = new Token('array', [tokenAddress1]);
        const tokens2 = new Token('array', [tokenAddress2]);
        const dynamic = new Token('array', [tokens1, tokens2]);
        const expected = `${padU32(0x20)}${padU32(2)}${padU32(0x80)}${padU32(
          0xc0
        )}${padU32(1)}${encAddress1}${padU32(1)}${encAddress2}`;

        expect(Encoder.encode([dynamic])).toEqual(expected);
      });

      it('encodes dynamic array of dynamic array addresses (2)', () => {
        const tokens1 = new Token('array', [tokenAddress1, tokenAddress2]);
        const tokens2 = new Token('array', [tokenAddress3, tokenAddress4]);
        const dynamic = new Token('array', [tokens1, tokens2]);
        const expected = `${padU32(0x20)}${padU32(2)}${padU32(0x80)}${padU32(
          0xe0
        )}${padU32(2)}${encAddress1}${encAddress2}${padU32(
          2
        )}${encAddress3}${encAddress4}`;

        expect(Encoder.encode([dynamic])).toEqual(expected);
      });

      it('encodes fixed array of fixed array addresses', () => {
        const tokens1 = new Token('fixedArray', [tokenAddress1, tokenAddress2]);
        const tokens2 = new Token('fixedArray', [tokenAddress3, tokenAddress4]);
        const dynamic = new Token('fixedArray', [tokens1, tokens2]);
        const expected = `${encAddress1}${encAddress2}${encAddress3}${encAddress4}`;

        expect(Encoder.encode([dynamic])).toEqual(expected);
      });
    });

    describe('bytes', () => {
      const bytes1 = '0x1234';
      const bytes2 =
        '0x10000000000000000000000000000000000000000000000000000000000002';
      const bytes3 =
        '0x1000000000000000000000000000000000000000000000000000000000000000';

      it('encodes fixed bytes', () => {
        const token = new Token('fixedBytes', bytes1);

        expect(Encoder.encode([token])).toEqual(padFixedBytes(bytes1));
      });

      it('encodes bytes', () => {
        const token = new Token('bytes', bytes1);

        expect(Encoder.encode([token])).toEqual(
          `${padU32(0x20)}${padU32(2)}${padFixedBytes(bytes1)}`
        );
      });

      it('encodes bytes (short of boundary)', () => {
        const token = new Token('bytes', bytes2);

        expect(Encoder.encode([token])).toEqual(
          `${padU32(0x20)}${padU32(0x1f)}${padFixedBytes(bytes2)}`
        );
      });

      it('encodes bytes (two blocks)', () => {
        const input = `${bytes3}${bytes3.slice(-64)}`;
        const token = new Token('bytes', input);

        expect(Encoder.encode([token])).toEqual(
          `${padU32(0x20)}${padU32(0x40)}${padFixedBytes(input)}`
        );
      });

      it('encodes two consecutive bytes', () => {
        const in1 =
          '0x10000000000000000000000000000000000000000000000000000000000002';
        const in2 =
          '0x0010000000000000000000000000000000000000000000000000000000000002';
        const tokens = [new Token('bytes', in1), new Token('bytes', in2)];

        expect(Encoder.encode(tokens)).toEqual(
          `${padU32(0x40)}${padU32(0x80)}${padU32(0x1f)}${padFixedBytes(
            in1
          )}${padU32(0x20)}${padFixedBytes(in2)}`
        );
      });
    });

    describe('string', () => {
      it('encodes a string', () => {
        const baseString = 'gavofyork';
        const stringEnc = padFixedBytes('0x6761766f66796f726b');
        const token = new Token('string', baseString);

        expect(Encoder.encode([token])).toEqual(
          `${padU32(0x20)}${padU32(baseString.length.toString(16))}${stringEnc}`
        );
      });
    });

    describe('uint', () => {
      it('encodes a uint', () => {
        const token = new Token('uint', 4);

        expect(Encoder.encode([token])).toEqual(padU32(4));
      });
    });

    describe('int', () => {
      it('encodes a int', () => {
        const token = new Token('int', 4);

        expect(Encoder.encode([token])).toEqual(padU32(4));
      });
    });

    describe('bool', () => {
      it('encodes a bool (true)', () => {
        const token = new Token('bool', true);

        expect(Encoder.encode([token])).toEqual(padU32(1));
      });

      it('encodes a bool (false)', () => {
        const token = new Token('bool', false);

        expect(Encoder.encode([token])).toEqual(padU32(0));
      });
    });

    describe('comprehensive test', () => {
      it('encodes a complex sequence', () => {
        const bytes =
          '0x131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b';
        const tokens = [
          new Token('int', 5),
          new Token('bytes', bytes),
          new Token('int', 3),
          new Token('bytes', bytes)
        ];

        expect(Encoder.encode(tokens)).toEqual(
          `${padU32(5)}${padU32(0x80)}${padU32(3)}${padU32(0xe0)}${padU32(
            0x40
          )}${bytes.substr(2)}${padU32(0x40)}${bytes.substr(2)}`
        );
      });

      it('encodes a complex sequence (nested)', () => {
        const array = [
          new Token('int', 5),
          new Token('int', 6),
          new Token('int', 7)
        ];
        const tokens = [
          new Token('int', 1),
          new Token('string', 'gavofyork'),
          new Token('int', 2),
          new Token('int', 3),
          new Token('int', 4),
          new Token('array', array)
        ];
        const stringEnc = padFixedBytes('0x6761766f66796f726b');

        expect(Encoder.encode(tokens)).toEqual(
          `${padU32(1)}${padU32(0xc0)}${padU32(2)}${padU32(3)}${padU32(
            4
          )}${padU32(0x100)}${padU32(9)}${stringEnc}${padU32(3)}${padU32(
            5
          )}${padU32(6)}${padU32(7)}`
        );
      });
    });
  });
});
