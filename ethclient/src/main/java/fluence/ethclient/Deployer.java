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

package fluence.ethclient;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.Callable;
import org.web3j.abi.EventEncoder;
import org.web3j.abi.TypeReference;
import org.web3j.abi.datatypes.DynamicArray;
import org.web3j.abi.datatypes.Event;
import org.web3j.abi.datatypes.Function;
import org.web3j.abi.datatypes.Type;
import org.web3j.abi.datatypes.generated.Bytes32;
import org.web3j.abi.datatypes.generated.Uint256;
import org.web3j.abi.datatypes.generated.Uint8;
import org.web3j.crypto.Credentials;
import org.web3j.protocol.Web3j;
import org.web3j.protocol.core.DefaultBlockParameter;
import org.web3j.protocol.core.RemoteCall;
import org.web3j.protocol.core.methods.request.EthFilter;
import org.web3j.protocol.core.methods.response.Log;
import org.web3j.protocol.core.methods.response.TransactionReceipt;
import org.web3j.tuples.generated.Tuple2;
import org.web3j.tx.Contract;
import org.web3j.tx.TransactionManager;
import org.web3j.tx.gas.ContractGasProvider;
import rx.Observable;
import rx.functions.Func1;

/**
 * <p>Auto generated code.
 * <p><strong>Do not modify!</strong>
 * <p>Please use the <a href="https://docs.web3j.io/command_line.html">web3j command line tools</a>,
 * or the org.web3j.codegen.SolidityFunctionWrapperGenerator in the 
 * <a href="https://github.com/web3j/web3j/tree/master/codegen">codegen module</a> to update.
 *
 * <p>Generated with web3j version 3.6.0.
 */
public class Deployer extends Contract {
    private static final String BINARY = "6080604052600160035534801561001557600080fd5b50610a11806100256000396000f300608060405260043610610062576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680632238ba2f146100675780632bb62ae0146100b35780634e69d560146100f2578063e8fe6b6614610124575b600080fd5b34801561007357600080fd5b506100b160048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190505050610180565b005b3480156100bf57600080fd5b506100f06004803603810190808035600019169060200190929190803560001916906020019092919050505061026b565b005b3480156100fe57600080fd5b50610107610407565b604051808381526020018281526020019250505060405180910390f35b34801561013057600080fd5b50610153600480360381019080803560001916906020019092919050505061041e565b60405180836000191660001916815260200182600019166000191681526020019250505060405180910390f35b600460606040519081016040528085600019168152602001846000191681526020018360ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff160217905550505050610220610558565b1515610266577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8360405180826000191660001916815260200191505060405180910390a15b505050565b6000600102600160008460001916600019168152602001908152602001600020600001546000191614151561032e576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260218152602001807f5468697320736f6c76657220697320616c72656164792072656769737465726581526020017f640000000000000000000000000000000000000000000000000000000000000081525060400191505060405180910390fd5b6040805190810160405280836000191681526020018260001916815250600160008460001916600019168152602001908152602001600020600082015181600001906000191690556020820151816001019060001916905590505060008290806001815401808255809150509060018203906000526020600020016000909192909190915090600019169055507fc6aa0f5a4d613646f930c3c4d9c6a4f213549c605897bf0ae5c92f914559a0118260405180826000191660001916815260200191505060405180910390a1610402610558565b505050565b600080600480549050600080549050915091509091565b60008061042961091a565b60026000856000191660001916815260200190815260200160002060408051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681525050815250509050600060010281600001516000191611151561053c576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b8060200151600001518160200151602001519250925050915091565b60008061056361093e565b60608060008060008096505b6004805490508710156105ca5760048781548110151561058b57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff166000805490501015156105bf576105ca565b86600101965061056f565b600480549050871015156105e15760009750610910565b6004878154811015156105f057fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250509550856040015160ff166040519080825280602002602001820160405280156106835781602001602082028038833980820191505090505b509450856040015160ff166040519080825280602002602001820160405280156106bc5781602001602082028038833980820191505090505b509350600092505b856040015160ff1683101561077257600060018460008054905003038154811015156106ec57fe5b9060005260206000200154915081858481518110151561070857fe5b90602001906020020190600019169081600019168152505060016000836000191660001916815260200190815260200160002060010154848481518110151561074d57fe5b90602001906020020190600019169081600019168152505082806001019350506106c4565b856040015160ff166000818180549050039150816107909190610969565b50600360008154809291906001019190505560010290506040805190810160405280826000191681526020018781525060026000836000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555050509050507f4c99ecb5fd2810b2ecde0e23958d1c8a5e626342d58fecc3fa9ce945887567048186866040518084600019166000191681526020018060200180602001838103835285818151815260200191508051906020019060200280838360005b838110156108b3578082015181840152602081019050610898565b50505050905001838103825284818151815260200191508051906020019060200280838360005b838110156108f55780820151818401526020810190506108da565b505050509050019550505050505060405180910390a1600197505b5050505050505090565b60806040519081016040528060008019168152602001610938610995565b81525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b8154818355818111156109905781836000526020600020918201910161098f91906109c0565b5b505050565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b6109e291905b808211156109de5760008160009055506001016109c6565b5090565b905600a165627a7a723058207ed2cff36d8d9d1eceebb282ff143fdb4368d27dcaf76015bc2290b400097e190029";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_ADDSOLVER = "addSolver";

    public static final String FUNC_GETSTATUS = "getStatus";

    public static final String FUNC_GETCODE = "getCode";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes32>>() {}));
    ;

    public static final Event CODEENQUEUED_EVENT = new Event("CodeEnqueued", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event NEWSOLVER_EVENT = new Event("NewSolver", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    @Deprecated
    protected Deployer(String contractAddress, Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        super(BINARY, contractAddress, web3j, credentials, gasPrice, gasLimit);
    }

    protected Deployer(String contractAddress, Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        super(BINARY, contractAddress, web3j, credentials, contractGasProvider);
    }

    @Deprecated
    protected Deployer(String contractAddress, Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        super(BINARY, contractAddress, web3j, transactionManager, gasPrice, gasLimit);
    }

    protected Deployer(String contractAddress, Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        super(BINARY, contractAddress, web3j, transactionManager, contractGasProvider);
    }

    public RemoteCall<TransactionReceipt> addCode(Bytes32 storageHash, Bytes32 storageReceipt, Uint8 clusterSize) {
        final Function function = new Function(
                FUNC_ADDCODE, 
                Arrays.<Type>asList(storageHash, storageReceipt, clusterSize), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> addSolver(Bytes32 solverID, Bytes32 solverAddress) {
        final Function function = new Function(
                FUNC_ADDSOLVER, 
                Arrays.<Type>asList(solverID, solverAddress), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Tuple2<Uint256, Uint256>> getStatus() {
        final Function function = new Function(FUNC_GETSTATUS, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Uint256>() {}, new TypeReference<Uint256>() {}));
        return new RemoteCall<Tuple2<Uint256, Uint256>>(
                new Callable<Tuple2<Uint256, Uint256>>() {
                    @Override
                    public Tuple2<Uint256, Uint256> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple2<Uint256, Uint256>(
                                (Uint256) results.get(0), 
                                (Uint256) results.get(1));
                    }
                });
    }

    public RemoteCall<Tuple2<Bytes32, Bytes32>> getCode(Bytes32 clusterID) {
        final Function function = new Function(FUNC_GETCODE, 
                Arrays.<Type>asList(clusterID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}));
        return new RemoteCall<Tuple2<Bytes32, Bytes32>>(
                new Callable<Tuple2<Bytes32, Bytes32>>() {
                    @Override
                    public Tuple2<Bytes32, Bytes32> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple2<Bytes32, Bytes32>(
                                (Bytes32) results.get(0), 
                                (Bytes32) results.get(1));
                    }
                });
    }

    public List<ClusterFormedEventResponse> getClusterFormedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CLUSTERFORMED_EVENT, transactionReceipt);
        ArrayList<ClusterFormedEventResponse> responses = new ArrayList<ClusterFormedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(0);
            typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(1);
            typedResponse.solverAddrs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(2);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<ClusterFormedEventResponse> clusterFormedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, ClusterFormedEventResponse>() {
            @Override
            public ClusterFormedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(CLUSTERFORMED_EVENT, log);
                ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
                typedResponse.log = log;
                typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(0);
                typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(1);
                typedResponse.solverAddrs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(2);
                return typedResponse;
            }
        });
    }

    public Observable<ClusterFormedEventResponse> clusterFormedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT));
        return clusterFormedEventObservable(filter);
    }

    public List<CodeEnqueuedEventResponse> getCodeEnqueuedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CODEENQUEUED_EVENT, transactionReceipt);
        ArrayList<CodeEnqueuedEventResponse> responses = new ArrayList<CodeEnqueuedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            CodeEnqueuedEventResponse typedResponse = new CodeEnqueuedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<CodeEnqueuedEventResponse> codeEnqueuedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, CodeEnqueuedEventResponse>() {
            @Override
            public CodeEnqueuedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(CODEENQUEUED_EVENT, log);
                CodeEnqueuedEventResponse typedResponse = new CodeEnqueuedEventResponse();
                typedResponse.log = log;
                typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<CodeEnqueuedEventResponse> codeEnqueuedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(CODEENQUEUED_EVENT));
        return codeEnqueuedEventObservable(filter);
    }

    public List<NewSolverEventResponse> getNewSolverEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(NEWSOLVER_EVENT, transactionReceipt);
        ArrayList<NewSolverEventResponse> responses = new ArrayList<NewSolverEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            NewSolverEventResponse typedResponse = new NewSolverEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<NewSolverEventResponse> newSolverEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, NewSolverEventResponse>() {
            @Override
            public NewSolverEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(NEWSOLVER_EVENT, log);
                NewSolverEventResponse typedResponse = new NewSolverEventResponse();
                typedResponse.log = log;
                typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<NewSolverEventResponse> newSolverEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(NEWSOLVER_EVENT));
        return newSolverEventObservable(filter);
    }

    public static RemoteCall<Deployer> deploy(Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        return deployRemoteCall(Deployer.class, web3j, credentials, contractGasProvider, BINARY, "");
    }

    @Deprecated
    public static RemoteCall<Deployer> deploy(Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        return deployRemoteCall(Deployer.class, web3j, credentials, gasPrice, gasLimit, BINARY, "");
    }

    public static RemoteCall<Deployer> deploy(Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        return deployRemoteCall(Deployer.class, web3j, transactionManager, contractGasProvider, BINARY, "");
    }

    @Deprecated
    public static RemoteCall<Deployer> deploy(Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        return deployRemoteCall(Deployer.class, web3j, transactionManager, gasPrice, gasLimit, BINARY, "");
    }

    @Deprecated
    public static Deployer load(String contractAddress, Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        return new Deployer(contractAddress, web3j, credentials, gasPrice, gasLimit);
    }

    @Deprecated
    public static Deployer load(String contractAddress, Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        return new Deployer(contractAddress, web3j, transactionManager, gasPrice, gasLimit);
    }

    public static Deployer load(String contractAddress, Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        return new Deployer(contractAddress, web3j, credentials, contractGasProvider);
    }

    public static Deployer load(String contractAddress, Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        return new Deployer(contractAddress, web3j, transactionManager, contractGasProvider);
    }

    public static class ClusterFormedEventResponse {
        public Log log;

        public Bytes32 clusterID;

        public DynamicArray<Bytes32> solverIDs;

        public DynamicArray<Bytes32> solverAddrs;
    }

    public static class CodeEnqueuedEventResponse {
        public Log log;

        public Bytes32 storageHash;
    }

    public static class NewSolverEventResponse {
        public Log log;

        public Bytes32 id;
    }
}
