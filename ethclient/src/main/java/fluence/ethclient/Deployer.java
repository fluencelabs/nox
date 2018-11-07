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
import org.web3j.abi.datatypes.generated.Int64;
import org.web3j.abi.datatypes.generated.Uint256;
import org.web3j.abi.datatypes.generated.Uint8;
import org.web3j.crypto.Credentials;
import org.web3j.protocol.Web3j;
import org.web3j.protocol.core.DefaultBlockParameter;
import org.web3j.protocol.core.RemoteCall;
import org.web3j.protocol.core.methods.request.EthFilter;
import org.web3j.protocol.core.methods.response.Log;
import org.web3j.protocol.core.methods.response.TransactionReceipt;
import org.web3j.tuples.generated.Tuple3;
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
    private static final String BINARY = "60806040526000600255600160055534801561001a57600080fd5b506110018061002a6000396000f300608060405260043610610062576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680632bb62ae0146100675780634e69d560146100a65780634e84e75814610126578063e2683e921461017f575b600080fd5b34801561007357600080fd5b506100a460048036038101908080356000191690602001909291908035600019169060200190929190505050610223565b005b3480156100b257600080fd5b506100bb610294565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156101105780820151818401526020810190506100f5565b5050505090500194505050505060405180910390f35b34801561013257600080fd5b5061017d60048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190803560070b9060200190929190505050610358565b005b34801561018b57600080fd5b506101ae600480360381019080803560001916906020019092919050505061047f565b604051808460001916600019168152602001836000191660001916815260200180602001828103825283818151815260200191508051906020019060200280838360005b8381101561020d5780820151818401526020810190506101f2565b5050505090500194505050505060405180910390f35b6102486040805190810160405280846000191681526020018360001916815250610641565b7fc6aa0f5a4d613646f930c3c4d9c6a4f213549c605897bf0ae5c92f914559a0118260405180826000191660001916815260200191505060405180910390a161028f6106ff565b505050565b6000806060806000806006805490506040519080825280602002602001820160405280156102d15781602001602082028038833980820191505090505b509250600091505b60068054905082101561033e576006828154811015156102f557fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff16838381518110151561032557fe5b90602001906020020181815250508160010191506102d9565b606590508060008054905084955095509550505050909192565b600660806040519081016040528086600019168152602001856000191681526020018460ff1681526020018360070b81525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555060608201518160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff1602179055505050506104336106ff565b1515610479577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8460405180826000191660001916815260200191505060405180910390a15b50505050565b600080606061048c610dd7565b6004600086600019166000191681526020019081526020016000206060604051908101604052908160008201546000191660001916815260200160018201608060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681526020016002820160019054906101000a900460070b60070b60070b8152505081526020016004820180548060200260200160405190810160405280929190818152602001828054801561058b57602002820191906000526020600020905b81546000191681526020019060010190808311610573575b50505050508152505090506000600102816000015160001916111515610619576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b8060200151600001518160200151602001518260400151809050935093509350509193909250565b6000819080600181540180825580915050906001820390600052602060002090600202016000909192909190915060008201518160000190600019169055602082015181600101906000191690555050506000600360008360000151600019166000191681526020019081526020016000205414156106cd576002600081548092919060010191905055505b600360008260000151600019166000191681526020019081526020016000206000815480929190600101919050555050565b60008061070a610e02565b606080600080600080600097505b6006805490508810156107705760068881548110151561073457fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1660025410151561076557610770565b876001019750610718565b600680549050881015156107875760009850610cae565b60068881548110151561079657fe5b9060005260206000209060030201608060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681526020016002820160019054906101000a900460070b60070b60070b815250509650600680549050600189011415156108f457600660016006805490500381548110151561083857fe5b906000526020600020906003020160068981548110151561085557fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055506002820160019054906101000a900460070b8160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff1602179055509050505b600680546001900390816109089190610e37565b50866040015160ff1660405190808252806020026020018201604052801561093f5781602001602082028038833980820191505090505b509550866040015160ff166040519080825280602002602001820160405280156109785781602001602082028038833980820191505090505b50945060056000815480929190600101919050556001029350866040015160ff169250600091505b600080549050821015610abd5760008314156109bb57610abd565b6000828154811015156109ca57fe5b906000526020600020906002020160000154905083600019166001600083600019166000191681526020019081526020016000205460001916141515610ab15783600160008360001916600019168152602001908152602001600020816000191690555082600190039250808684815181101515610a4457fe5b906020019060200201906000191690816000191681525050600082815481101515610a6b57fe5b9060005260206000209060020201600101548584815181101515610a8b57fe5b906020019060200201906000191690816000191681525050610aac82610cb9565b610ab8565b8160010191505b6109a0565b600083141515610ac957fe5b606060405190810160405280856000191681526020018881526020018781525060046000866000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555060608201518160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff16021790555050506040820151816004019080519060200190610bae929190610e69565b509050507ffaf3fa3d363b6dc43e4c2e2cb2951b2de8ba1f3b4ab1b17843a2980fd3b3d878876000015188606001518689896040518086600019166000191681526020018560070b60070b815260200184600019166000191681526020018060200180602001838103835285818151815260200191508051906020019060200280838360005b83811015610c4f578082015181840152602081019050610c34565b50505050905001838103825284818151815260200191508051906020019060200280838360005b83811015610c91578082015181840152602081019050610c76565b5050505090500197505050505050505060405180910390a1600198505b505050505050505090565b60008082815481101515610cc957fe5b9060005260206000209060020201600001549050600060036000836000191660001916815260200190815260200160002054111515610d0457fe5b600060036000836000191660001916815260200190815260200160002060008154600190039190508190551415610d475760026000815460019003919050819055505b60008054905060018301141515610dbe576000600160008054905003815481101515610d6f57fe5b9060005260206000209060020201600083815481101515610d8c57fe5b906000526020600020906002020160008201548160000190600019169055600182015481600101906000191690559050505b60008054600190039081610dd29190610ebc565b505050565b60c06040519081016040528060008019168152602001610df5610eee565b8152602001606081525090565b6080604051908101604052806000801916815260200160008019168152602001600060ff168152602001600060070b81525090565b815481835581811115610e6457600302816003028360005260206000209182019101610e639190610f23565b5b505050565b828054828255906000526020600020908101928215610eab579160200282015b82811115610eaa578251829060001916905591602001919060010190610e89565b5b509050610eb89190610f81565b5090565b815481835581811115610ee957600202816002028360005260206000209182019101610ee89190610fa6565b5b505050565b6080604051908101604052806000801916815260200160008019168152602001600060ff168152602001600060070b81525090565b610f7e91905b80821115610f7a5760008082016000905560018201600090556002820160006101000a81549060ff02191690556002820160016101000a81549067ffffffffffffffff021916905550600301610f29565b5090565b90565b610fa391905b80821115610f9f576000816000905550600101610f87565b5090565b90565b610fd291905b80821115610fce57600080820160009055600182016000905550600201610fac565b5090565b905600a165627a7a723058201f20f729edac6906030d0039c8e4cac161f82518a4b754f5075ee1d3666caf9f0029";

    public static final String FUNC_ADDSOLVER = "addSolver";

    public static final String FUNC_GETSTATUS = "getStatus";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_GETCLUSTER = "getCluster";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Int64>() {}, new TypeReference<Bytes32>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes32>>() {}));
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

    public RemoteCall<TransactionReceipt> addSolver(Bytes32 solverID, Bytes32 solverAddress) {
        final Function function = new Function(
                FUNC_ADDSOLVER, 
                Arrays.<Type>asList(solverID, solverAddress), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>> getStatus() {
        final Function function = new Function(FUNC_GETSTATUS, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Uint8>() {}, new TypeReference<Uint256>() {}, new TypeReference<DynamicArray<Uint256>>() {}));
        return new RemoteCall<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>>(
                new Callable<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>>() {
                    @Override
                    public Tuple3<Uint8, Uint256, DynamicArray<Uint256>> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple3<Uint8, Uint256, DynamicArray<Uint256>>(
                                (Uint8) results.get(0), 
                                (Uint256) results.get(1), 
                                (DynamicArray<Uint256>) results.get(2));
                    }
                });
    }

    public RemoteCall<TransactionReceipt> addCode(Bytes32 storageHash, Bytes32 storageReceipt, Uint8 clusterSize, Int64 genesisTime) {
        final Function function = new Function(
                FUNC_ADDCODE, 
                Arrays.<Type>asList(storageHash, storageReceipt, clusterSize, genesisTime), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Tuple3<Bytes32, Bytes32, DynamicArray<Bytes32>>> getCluster(Bytes32 clusterID) {
        final Function function = new Function(FUNC_GETCLUSTER, 
                Arrays.<Type>asList(clusterID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<DynamicArray<Bytes32>>() {}));
        return new RemoteCall<Tuple3<Bytes32, Bytes32, DynamicArray<Bytes32>>>(
                new Callable<Tuple3<Bytes32, Bytes32, DynamicArray<Bytes32>>>() {
                    @Override
                    public Tuple3<Bytes32, Bytes32, DynamicArray<Bytes32>> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple3<Bytes32, Bytes32, DynamicArray<Bytes32>>(
                                (Bytes32) results.get(0), 
                                (Bytes32) results.get(1), 
                                (DynamicArray<Bytes32>) results.get(2));
                    }
                });
    }

    public List<ClusterFormedEventResponse> getClusterFormedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CLUSTERFORMED_EVENT, transactionReceipt);
        ArrayList<ClusterFormedEventResponse> responses = new ArrayList<ClusterFormedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
            typedResponse.genesisTime = (Int64) eventValues.getNonIndexedValues().get(1);
            typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(2);
            typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
            typedResponse.solverAddrs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(4);
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
                typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
                typedResponse.genesisTime = (Int64) eventValues.getNonIndexedValues().get(1);
                typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(2);
                typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
                typedResponse.solverAddrs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(4);
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

        public Bytes32 storageHash;

        public Int64 genesisTime;

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
