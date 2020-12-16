/* tslint:disable */
/* eslint-disable */
/**
 * @param wasm
 * @param {string} init_user_id
 * @param {string} aqua
 * @param {string} prev_data
 * @param {string} data
 * @param {string} log_level
 * @returns {string}
 */
export function invoke(wasm: any, init_user_id: string, aqua: string, prev_data: string, data: string, log_level: string): string;
export function ast(wasm: any, script: string): string;
export function return_current_peer_id(wasm: any, peerId: string, arg0: any): void;
export function return_call_service_result(wasm: any, ret: string, arg0: any, arg1: any, arg2: any, arg3: any, arg4: any, arg5: any, arg6: any): void;
export function getStringFromWasm0(wasm: any, arg1: any, arg2: any): string

