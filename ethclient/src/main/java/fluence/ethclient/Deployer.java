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
    private static final String BINARY = "60806040526000600355600160055534801561001a57600080fd5b50610ee98061002a6000396000f300608060405260043610610062576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680632238ba2f146100675780632bb62ae0146100b35780634e69d560146100f2578063e2683e9214610172575b600080fd5b34801561007357600080fd5b506100b160048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190505050610216565b005b3480156100bf57600080fd5b506100f060048036038101908080356000191690602001909291908035600019169060200190929190505050610301565b005b3480156100fe57600080fd5b5061010761041c565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b8381101561015c578082015181840152602081019050610141565b5050505090500194505050505060405180910390f35b34801561017e57600080fd5b506101a160048036038101908080356000191690602001909291905050506104e0565b604051808460001916600019168152602001836000191660001916815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156102005780820151818401526020810190506101e5565b5050505090500194505050505060405180910390f35b600660606040519081016040528085600019168152602001846000191681526020018360ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff1602179055505050506102b6610685565b15156102fc577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8360405180826000191660001916815260200191505060405180910390a15b505050565b6000604080519081016040528084600019168152602001836000191681525090806001815401808255809150509060018203906000526020600020906002020160009091929091909150600082015181600001906000191690556020820151816001019060001916905550505060006002600084600019166000191681526020019081526020016000205414156103a5576003600081548092919060010191905055505b600260008360001916600019168152602001908152602001600020600081548092919060010191905055507fc6aa0f5a4d613646f930c3c4d9c6a4f213549c605897bf0ae5c92f914559a0118260405180826000191660001916815260200191505060405180910390a1610417610685565b505050565b6000806060806000806006805490506040519080825280602002602001820160405280156104595781602001602082028038833980820191505090505b509250600091505b6006805490508210156104c65760068281548110151561047d57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1683838151811015156104ad57fe5b9060200190602002018181525050816001019150610461565b606590508060008054905084955095509550505050909192565b60008060606104ed610cee565b6004600086600019166000191681526020019081526020016000206060604051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250508152602001600482018054806020026020016040519081016040528092919081815260200182805480156105cf57602002820191906000526020600020905b815460001916815260200190600101908083116105b7575b5050505050815250509050600060010281600001516000191611151561065d576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b8060200151600001518160200151602001518260400151809050935093509350509193909250565b600080610690610d19565b60608060008060008096505b6006805490508710156106f4576006878154811015156106b857fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff166003541015156106e9576106f4565b86600101965061069c565b6006805490508710151561070b5760009750610ce4565b60068781548110151561071a57fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff168152505095506006805490506001880114151561081c57600660016006805490500381548110151561079f57fe5b90600052602060002090600302016006888154811015156107bc57fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055509050505b600680546001900390816108309190610d44565b50856040015160ff166040519080825280602002602001820160405280156108675781602001602082028038833980820191505090505b509450856040015160ff166040519080825280602002602001820160405280156108a05781602001602082028038833980820191505090505b50935060056000815480929190600101919050556001029250856040015160ff169150600090505b600080549050811015610b4b5760008214156108e357610b4b565b82600019166001600080848154811015156108fa57fe5b906000526020600020906002020160000154600019166000191681526020019081526020016000205460001916141515610b3f578160019003915060008181548110151561094457fe5b906000526020600020906002020160000154858381518110151561096457fe5b90602001906020020190600019169081600019168152505060008181548110151561098b57fe5b90600052602060002090600202016001015484838151811015156109ab57fe5b906020019060200201906000191690816000191681525050826001600080848154811015156109d657fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002081600019169055506000600260008084815481101515610a1a57fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002054111515610a4c57fe5b6000600260008084815481101515610a6057fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002060008154600190039190508190551415610aae5760036000815460019003919050819055505b60008054905060018201141515610b25576000600160008054905003815481101515610ad657fe5b9060005260206000209060020201600082815481101515610af357fe5b906000526020600020906002020160008201548160000190600019169055600182015481600101906000191690559050505b60008054600190039081610b399190610d76565b50610b46565b8060010190505b6108c8565b600082141515610b5757fe5b606060405190810160405280846000191681526020018781526020018681525060046000856000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555050506040820151816004019080519060200190610c0a929190610da8565b509050507f4c99ecb5fd2810b2ecde0e23958d1c8a5e626342d58fecc3fa9ce945887567048386866040518084600019166000191681526020018060200180602001838103835285818151815260200191508051906020019060200280838360005b83811015610c87578082015181840152602081019050610c6c565b50505050905001838103825284818151815260200191508051906020019060200280838360005b83811015610cc9578082015181840152602081019050610cae565b505050509050019550505050505060405180910390a1600197505b5050505050505090565b60a06040519081016040528060008019168152602001610d0c610dfb565b8152602001606081525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b815481835581811115610d7157600302816003028360005260206000209182019101610d709190610e26565b5b505050565b815481835581811115610da357600202816002028360005260206000209182019101610da29190610e69565b5b505050565b828054828255906000526020600020908101928215610dea579160200282015b82811115610de9578251829060001916905591602001919060010190610dc8565b5b509050610df79190610e98565b5090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b610e6691905b80821115610e625760008082016000905560018201600090556002820160006101000a81549060ff021916905550600301610e2c565b5090565b90565b610e9591905b80821115610e9157600080820160009055600182016000905550600201610e6f565b5090565b90565b610eba91905b80821115610eb6576000816000905550600101610e9e565b5090565b905600a165627a7a7230582096754f2315bf11f7f27891150a65e113de606e25efe9763911ff36e6b8b1171d0029";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_ADDSOLVER = "addSolver";

    public static final String FUNC_GETSTATUS = "getStatus";

    public static final String FUNC_GETCLUSTER = "getCluster";

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
