/* tslint:disable */
/* eslint-disable */
/**
 * @param wasm
 * @param {string} init_user_id
 * @param {string} aqua
 * @param {string} data
 * @returns {string}
 */
export function invoke(wasm: any, init_user_id: string, aqua: string, data: string): string;
export function getStringFromWasm0(wasm: any, arg1: any, arg2: any): string
export function getInt32Memory0(wasm: any): number[]
export function passStringToWasm0(wasm: any, arg: any, malloc: any, realloc: any): number
export let WASM_VECTOR_LEN: number;
