// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { fromParamType, toParamType } from './format';
import ParamType from './paramType';
import { TokenTypeEnum } from '../../types';

describe('spec/paramType/format', () => {
  describe('fromParamType', () => {
    it('errors on invalid types', () => {
      expect(() =>
        fromParamType({
          type: 'noMatch'
        } as any)
      ).toThrow(/noMatch/);
    });

    describe('simple types', () => {
      it('converts address to address', () => {
        const pt = new ParamType('address');

        expect(fromParamType(pt)).toEqual('address');
      });

      it('converts bool to bool', () => {
        const pt = new ParamType('bool');

        expect(fromParamType(pt)).toEqual('bool');
      });

      it('converts bytes to bytes', () => {
        const pt = new ParamType('bytes');

        expect(fromParamType(pt)).toEqual('bytes');
      });

      it('converts string to string', () => {
        const pt = new ParamType('string');

        expect(fromParamType(pt)).toEqual('string');
      });
    });

    describe('length types', () => {
      it('converts int32 to int32', () => {
        const pt = new ParamType('int', undefined, 32);

        expect(fromParamType(pt)).toEqual('int32');
      });

      it('converts uint64 to int64', () => {
        const pt = new ParamType('uint', undefined, 64);

        expect(fromParamType(pt)).toEqual('uint64');
      });

      it('converts fixedBytes8 to bytes8', () => {
        const pt = new ParamType('fixedBytes', undefined, 8);

        expect(fromParamType(pt)).toEqual('bytes8');
      });
    });

    describe('arrays', () => {
      it('converts string[2] to string[2]', () => {
        const pt = new ParamType('fixedArray', new ParamType('string'), 2);

        expect(fromParamType(pt)).toEqual('string[2]');
      });

      it('converts bool[] to bool[]', () => {
        const pt = new ParamType('array', new ParamType('bool'));

        expect(fromParamType(pt)).toEqual('bool[]');
      });

      it('converts bool[][2] to bool[][2]', () => {
        const pt = new ParamType(
          'fixedArray',
          new ParamType('array', new ParamType('bool')),
          2
        );

        expect(fromParamType(pt)).toEqual('bool[][2]');
      });

      it('converts bool[2][] to bool[2][]', () => {
        const pt = new ParamType(
          'array',
          new ParamType('fixedArray', new ParamType('bool'), 2)
        );

        expect(fromParamType(pt)).toEqual('bool[2][]');
      });
    });
  });

  describe('toParamType', () => {
    it('errors on invalid types', () => {
      expect(() => toParamType('noMatch' as TokenTypeEnum)).toThrow(/noMatch/);
    });

    describe('simple mapping', () => {
      it('converts address to address', () => {
        const pt = toParamType('address');

        expect(pt.type).toEqual('address');
      });

      it('converts bool to bool', () => {
        const pt = toParamType('bool');

        expect(pt.type).toEqual('bool');
      });

      it('converts bytes to bytes', () => {
        const pt = toParamType('bytes');

        expect(pt.type).toEqual('bytes');
      });

      it('converts string to string', () => {
        const pt = toParamType('string');

        expect(pt.type).toEqual('string');
      });
    });

    describe('number', () => {
      it('converts int to int256', () => {
        const pt = toParamType('int');

        expect(pt.type).toEqual('int');
        expect(pt.length).toEqual(256);
      });

      it('converts uint to uint256', () => {
        const pt = toParamType('uint');

        expect(pt.type).toEqual('uint');
        expect(pt.length).toEqual(256);
      });
    });

    describe('sized types', () => {
      it('converts int32 to int32', () => {
        const pt = toParamType('int32');

        expect(pt.type).toEqual('int');
        expect(pt.length).toEqual(32);
      });

      it('converts uint16 to uint16', () => {
        const pt = toParamType('uint32');

        expect(pt.type).toEqual('uint');
        expect(pt.length).toEqual(32);
      });

      it('converts bytes8 to fixedBytes8', () => {
        const pt = toParamType('bytes8');

        expect(pt.type).toEqual('fixedBytes');
        expect(pt.length).toEqual(8);
      });
    });

    describe('arrays', () => {
      describe('fixed arrays', () => {
        it('creates fixed array', () => {
          const pt = toParamType('bytes[8]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('fixedArray');
          expect(pt.subtype.type).toEqual('bytes');
          expect(pt.length).toEqual(8);
        });

        it('creates fixed arrays of fixed arrays', () => {
          const pt = toParamType('bytes[45][3]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('fixedArray');
          expect(pt.length).toEqual(3);
          expect(pt.subtype.type).toEqual('fixedArray');
          expect(pt.subtype.length).toEqual(45);

          if (!pt.subtype.subtype) {
            throw new Error('No subtype.');
          }
          expect(pt.subtype.subtype.type).toEqual('bytes');
        });
      });

      describe('dynamic arrays', () => {
        it('creates a dynamic array', () => {
          const pt = toParamType('bytes[]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('array');
          expect(pt.subtype.type).toEqual('bytes');
        });

        it('creates a dynamic array of dynamic arrays', () => {
          const pt = toParamType('bool[][]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('array');
          expect(pt.subtype.type).toEqual('array');

          if (!pt.subtype.subtype) {
            throw new Error('No subtype.');
          }
          expect(pt.subtype.subtype.type).toEqual('bool');
        });
      });

      describe('mixed arrays', () => {
        it('creates a fixed dynamic array', () => {
          const pt = toParamType('bool[][3]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('fixedArray');
          expect(pt.length).toEqual(3);
          expect(pt.subtype.type).toEqual('array');

          if (!pt.subtype.subtype) {
            throw new Error('No subtype.');
          }
          expect(pt.subtype.subtype.type).toEqual('bool');
        });

        it('creates a dynamic fixed array', () => {
          const pt = toParamType('bool[3][]');
          if (!pt.subtype) {
            throw new Error('No subtype.');
          }

          expect(pt.type).toEqual('array');
          expect(pt.subtype.type).toEqual('fixedArray');
          expect(pt.subtype.length).toEqual(3);

          if (!pt.subtype.subtype) {
            throw new Error('No subtype.');
          }
          expect(pt.subtype.subtype.type).toEqual('bool');
        });
      });
    });
  });
});
