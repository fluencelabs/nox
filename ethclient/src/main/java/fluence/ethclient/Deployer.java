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
    private static final String BINARY = "60806040526000600355600160055534801561001a57600080fd5b506110328061002a6000396000f300608060405260043610610062576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680632bb62ae0146100675780634e69d560146100a65780634e84e75814610126578063e2683e921461017f575b600080fd5b34801561007357600080fd5b506100a460048036038101908080356000191690602001909291908035600019169060200190929190505050610223565b005b3480156100b257600080fd5b506100bb61033e565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156101105780820151818401526020810190506100f5565b5050505090500194505050505060405180910390f35b34801561013257600080fd5b5061017d60048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190803560070b9060200190929190505050610402565b005b34801561018b57600080fd5b506101ae6004803603810190808035600019169060200190929190505050610529565b604051808460001916600019168152602001836000191660001916815260200180602001828103825283818151815260200191508051906020019060200280838360005b8381101561020d5780820151818401526020810190506101f2565b5050505090500194505050505060405180910390f35b6000604080519081016040528084600019168152602001836000191681525090806001815401808255809150509060018203906000526020600020906002020160009091929091909150600082015181600001906000191690556020820151816001019060001916905550505060006002600084600019166000191681526020019081526020016000205414156102c7576003600081548092919060010191905055505b600260008360001916600019168152602001908152602001600020600081548092919060010191905055507fc6aa0f5a4d613646f930c3c4d9c6a4f213549c605897bf0ae5c92f914559a0118260405180826000191660001916815260200191505060405180910390a16103396106eb565b505050565b60008060608060008060068054905060405190808252806020026020018201604052801561037b5781602001602082028038833980820191505090505b509250600091505b6006805490508210156103e85760068281548110151561039f57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1683838151811015156103cf57fe5b9060200190602002018181525050816001019150610383565b606590508060008054905084955095509550505050909192565b600660806040519081016040528086600019168152602001856000191681526020018460ff1681526020018360070b81525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555060608201518160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff1602179055505050506104dd6106eb565b1515610523577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8460405180826000191660001916815260200191505060405180910390a15b50505050565b6000806060610536610e08565b6004600086600019166000191681526020019081526020016000206060604051908101604052908160008201546000191660001916815260200160018201608060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681526020016002820160019054906101000a900460070b60070b60070b8152505081526020016004820180548060200260200160405190810160405280929190818152602001828054801561063557602002820191906000526020600020905b8154600019168152602001906001019080831161061d575b505050505081525050905060006001028160000151600019161115156106c3576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b8060200151600001518160200151602001518260400151809050935093509350509193909250565b6000806106f6610e33565b60608060008060008096505b60068054905087101561075a5760068781548110151561071e57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1660035410151561074f5761075a565b866001019650610702565b600680549050871015156107715760009750610dfe565b60068781548110151561078057fe5b9060005260206000209060030201608060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681526020016002820160019054906101000a900460070b60070b60070b815250509550600680549050600188011415156108de57600660016006805490500381548110151561082257fe5b906000526020600020906003020160068881548110151561083f57fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055506002820160019054906101000a900460070b8160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff1602179055509050505b600680546001900390816108f29190610e68565b50856040015160ff166040519080825280602002602001820160405280156109295781602001602082028038833980820191505090505b509450856040015160ff166040519080825280602002602001820160405280156109625781602001602082028038833980820191505090505b50935060056000815480929190600101919050556001029250856040015160ff169150600090505b600080549050811015610c0d5760008214156109a557610c0d565b82600019166001600080848154811015156109bc57fe5b906000526020600020906002020160000154600019166000191681526020019081526020016000205460001916141515610c015781600190039150600081815481101515610a0657fe5b9060005260206000209060020201600001548583815181101515610a2657fe5b906020019060200201906000191690816000191681525050600081815481101515610a4d57fe5b9060005260206000209060020201600101548483815181101515610a6d57fe5b90602001906020020190600019169081600019168152505082600160008084815481101515610a9857fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002081600019169055506000600260008084815481101515610adc57fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002054111515610b0e57fe5b6000600260008084815481101515610b2257fe5b9060005260206000209060020201600001546000191660001916815260200190815260200160002060008154600190039190508190551415610b705760036000815460019003919050819055505b60008054905060018201141515610be7576000600160008054905003815481101515610b9857fe5b9060005260206000209060020201600082815481101515610bb557fe5b906000526020600020906002020160008201548160000190600019169055600182015481600101906000191690559050505b60008054600190039081610bfb9190610e9a565b50610c08565b8060010190505b61098a565b600082141515610c1957fe5b606060405190810160405280846000191681526020018781526020018681525060046000856000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555060608201518160020160016101000a81548167ffffffffffffffff021916908360070b67ffffffffffffffff16021790555050506040820151816004019080519060200190610cfe929190610ecc565b509050507ffaf3fa3d363b6dc43e4c2e2cb2951b2de8ba1f3b4ab1b17843a2980fd3b3d878866000015187606001518588886040518086600019166000191681526020018560070b60070b815260200184600019166000191681526020018060200180602001838103835285818151815260200191508051906020019060200280838360005b83811015610d9f578082015181840152602081019050610d84565b50505050905001838103825284818151815260200191508051906020019060200280838360005b83811015610de1578082015181840152602081019050610dc6565b5050505090500197505050505050505060405180910390a1600197505b5050505050505090565b60c06040519081016040528060008019168152602001610e26610f1f565b8152602001606081525090565b6080604051908101604052806000801916815260200160008019168152602001600060ff168152602001600060070b81525090565b815481835581811115610e9557600302816003028360005260206000209182019101610e949190610f54565b5b505050565b815481835581811115610ec757600202816002028360005260206000209182019101610ec69190610fb2565b5b505050565b828054828255906000526020600020908101928215610f0e579160200282015b82811115610f0d578251829060001916905591602001919060010190610eec565b5b509050610f1b9190610fe1565b5090565b6080604051908101604052806000801916815260200160008019168152602001600060ff168152602001600060070b81525090565b610faf91905b80821115610fab5760008082016000905560018201600090556002820160006101000a81549060ff02191690556002820160016101000a81549067ffffffffffffffff021916905550600301610f5a565b5090565b90565b610fde91905b80821115610fda57600080820160009055600182016000905550600201610fb8565b5090565b90565b61100391905b80821115610fff576000816000905550600101610fe7565b5090565b905600a165627a7a7230582059ff3c8b96de3a7fea7edc65191ca928627ae85e797f6a6e69d024ee7d73ffda0029";

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
