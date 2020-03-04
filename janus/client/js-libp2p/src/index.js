'use strict';

import { start_peer } from "./peer.js";

const connect_btn_name = "connect";
const send_btn_name = "send";
const addr_field_name = "addr";
const output_field_name = "output";
const status_field_name = "status";
const msg_field_name = "message";

var send_msg_callback = null;

async function connect() {
    const output_field = document.getElementById(output_field_name);
    const status_field = document.getElementById(status_field_name);
    const addr_field = document.getElementById(addr_field_name);

    const print_incoming = (txt) => {
        console.info(txt);
        output_field.innerText = `${txt.trim()}\n`;
    };

    const print_status = (txt) => {
        console.info(txt);
    };

    send_msg_callback = await start_peer(addr_field.value, print_status, print_incoming);
}

function send_message() {
    if(send_msg_callback == null) {
        // callback hasn't been send yet
        return;
    }

    const msg_field = document.getElementById(msg_field_name);
    send_msg_callback(msg_field.value);
}

const connect_btn = document.getElementById(connect_btn_name);
connect_btn.addEventListener("click", connect, false);

const send_btn = document.getElementById(send_btn_name);
send_btn.addEventListener("click", send_message, false);
