/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package org.web3j.protocol.core.methods.request;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.web3j.protocol.core.DefaultBlockParameter;

import java.util.List;

public class SingleAddressEthFilter extends EthFilter {
    public SingleAddressEthFilter(
            DefaultBlockParameter fromBlock,
            DefaultBlockParameter toBlock,
                     String address) {
        super(fromBlock, toBlock, address);
    }

    @Override
    public DefaultBlockParameter getFromBlock() {
        return super.getFromBlock();
    }

    @Override
    public DefaultBlockParameter getToBlock() {
        return super.getToBlock();
    }

    @JsonIgnore
    @Override
    public List<String> getAddress() {
        return super.getAddress();
    }

    @JsonProperty("address")
    public String getSingleAddress() {
        return super.getAddress().get(0);
    }

    @Override
    EthFilter getThis() {
        return super.getThis();
    }
}
