package fluence.ethclient;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.Callable;
import org.web3j.abi.EventEncoder;
import org.web3j.abi.TypeReference;
import org.web3j.abi.datatypes.Address;
import org.web3j.abi.datatypes.Bool;
import org.web3j.abi.datatypes.DynamicArray;
import org.web3j.abi.datatypes.Event;
import org.web3j.abi.datatypes.Function;
import org.web3j.abi.datatypes.Type;
import org.web3j.abi.datatypes.Utf8String;
import org.web3j.abi.datatypes.generated.Bytes24;
import org.web3j.abi.datatypes.generated.Bytes32;
import org.web3j.abi.datatypes.generated.Int64;
import org.web3j.abi.datatypes.generated.Uint16;
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
import org.web3j.tuples.generated.Tuple5;
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
    private static final String BINARY = "60806040526001600755336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506123b3806100586000396000f3006080604052600436106100db576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630988ca8c146100e057806318b919e914610169578063217fe6c6146101f95780632238ba2f1461029a57806324953eaa146102e6578063286dd3f51461034c5780634e69d5601461038f578063715018a61461040f5780637b9417c8146104265780637ea29f62146104695780638da5cb5b146104cb5780639b19251a14610522578063e2683e921461057d578063e2ec6ec3146106b1578063f2fde38b14610717575b600080fd5b3480156100ec57600080fd5b50610167600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f016020809104026020016040519081016040528093929190818152602001838380828437820191505050505050919291929050505061075a565b005b34801561017557600080fd5b5061017e6107db565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156101be5780820151818401526020810190506101a3565b50505050905090810190601f1680156101eb5780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561020557600080fd5b50610280600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f0160208091040260200160405190810160405280939291908181526020018383808284378201915050505050509192919290505050610814565b604051808215151515815260200191505060405180910390f35b3480156102a657600080fd5b506102e460048036038101908080356000191690602001909291908035600019169060200190929190803560ff16906020019092919050505061089b565b005b3480156102f257600080fd5b5061034a6004803603810190808035906020019082018035906020019080806020026020016040519081016040528093929190818152602001838360200280828437820191505050505050919291929050505061099b565b005b34801561035857600080fd5b5061038d600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610a37565b005b34801561039b57600080fd5b506103a4610ad4565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156103f95780820151818401526020810190506103de565b5050505090500194505050505060405180910390f35b34801561041b57600080fd5b50610424610b98565b005b34801561043257600080fd5b50610467600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610c9a565b005b34801561047557600080fd5b506104c96004803603810190808035600019169060200190929190803567ffffffffffffffff19169060200190929190803561ffff169060200190929190803561ffff169060200190929190505050610d37565b005b3480156104d757600080fd5b506104e0611036565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561052e57600080fd5b50610563600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061105b565b604051808215151515815260200191505060405180910390f35b34801561058957600080fd5b506105ac60048036038101908080356000191690602001909291905050506110a3565b6040518086600019166000191681526020018560001916600019168152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b838110156106135780820151818401526020810190506105f8565b50505050905001848103835286818151815260200191508051906020019060200280838360005b8381101561065557808201518184015260208101905061063a565b50505050905001848103825285818151815260200191508051906020019060200280838360005b8381101561069757808201518184015260208101905061067c565b505050509050019850505050505050505060405180910390f35b3480156106bd57600080fd5b506107156004803603810190808035906020019082018035906020019080806020026020016040519081016040528093929190818152602001838360200280828437820191505050505050919291929050505061140e565b005b34801561072357600080fd5b50610758600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506114aa565b005b6107d7826001836040518082805190602001908083835b6020831015156107965780518252602082019150602081019050602083039250610771565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061151190919063ffffffff16565b5050565b6040805190810160405280600981526020017f77686974656c697374000000000000000000000000000000000000000000000081525081565b6000610893836001846040518082805190602001908083835b602083101515610852578051825260208201915060208101905060208303925061082d565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061152a90919063ffffffff16565b905092915050565b6108a43361105b565b15156108af57610996565b600860606040519081016040528085600019168152602001846000191681526020018360ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555050505061094f611583565b1515610995577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8360405180826000191660001916815260200191505060405180910390a15b5b505050565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161415156109f857600080fd5b600090505b8151811015610a3357610a268282815181101515610a1757fe5b90602001906020020151610a37565b80806001019150506109fd565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610a9257600080fd5b610ad1816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611bc8565b50565b600080606080600080600880549050604051908082528060200260200182016040528015610b115781602001602082028038833980820191505090505b509250600091505b600880549050821015610b7e57600882815481101515610b3557fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff168383815181101515610b6557fe5b9060200190602002018181525050816001019150610b19565b606590508060028054905084955095509550505050909192565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610bf357600080fd5b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167ff8df31144d9c2f0f6b59d69b8b98abd5459d07f2742c4df920b25aae33c6482060405160405180910390a260008060006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610cf557600080fd5b610d34816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611cfc565b50565b610d403361105b565b1515610d4b57611030565b60006001026003600086600019166000191681526020019081526020016000206000015460001916141515610de8576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601f8152602001807f54686973206e6f646520697320616c726561647920726567697374657265640081525060200191505060405180910390fd5b8061ffff168261ffff16101515610e67576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260208152602001807f506f72742072616e676520697320656d707479206f7220696e636f727265637481525060200191505060405180910390fd5b60c060405190810160405280856000191681526020018467ffffffffffffffff191681526020018361ffff1681526020018261ffff1681526020018361ffff1681526020016006805490508152506003600086600019166000191681526020019081526020016000206000820151816000019060001916905560208201518160010160006101000a81548177ffffffffffffffffffffffffffffffffffffffffffffffff0219169083680100000000000000009004021790555060408201518160010160186101000a81548161ffff021916908361ffff160217905550606082015181600101601a6101000a81548161ffff021916908361ffff160217905550608082015181600101601c6101000a81548161ffff021916908361ffff16021790555060a08201518160020155905050600284908060018154018082558091505090600182039060005260206000200160009091929091909150906000191690555081810361ffff16600681818054905001915081610fe691906121ce565b507fb0cd47a7093fb93a9ce97304d3afb8df43e02e48502e47fd5fbb6c4020d935b58460405180826000191660001916815260200191505060405180910390a161102e611583565b505b50505050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600061109c826040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250610814565b9050919050565b60008060608060606110b36121fa565b606080606060006110c2612225565b600560008d600019166000191681526020019081526020016000206060604051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250508152602001600482015481525050955060006001028660000151600019161115156111e0576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b85602001516040015160ff1660405190808252806020026020018201604052801561121a5781602001602082028038833980820191505090505b50945085602001516040015160ff166040519080825280602002602001820160405280156112575781602001602082028038833980820191505090505b50935085602001516040015160ff166040519080825280602002602001820160405280156112945781602001602082028038833980820191505090505b509250600091505b85602001516040015160ff168210156113e0576004828760400151018154811015156112c457fe5b90600052602060002090600202016040805190810160405290816000820154600019166000191681526020016001820160009054906101000a900461ffff1661ffff1661ffff168152505090508060000151858381518110151561132457fe5b9060200190602002019060001916908160001916815250506003600082600001516000191660001916815260200190815260200160002060010160009054906101000a90046801000000000000000002848381518110151561138257fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff191681525050806020015183838151811015156113bb57fe5b9060200190602002019061ffff16908161ffff1681525050818060010192505061129c565b8560200151600001518660200151602001518686869a509a509a509a509a5050505050505091939590929450565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561146b57600080fd5b600090505b81518110156114a657611499828281518110151561148a57fe5b90602001906020020151610c9a565b8080600101915050611470565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561150557600080fd5b61150e81611e30565b50565b61151b828261152a565b151561152657600080fd5b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060009054906101000a900460ff16905092915050565b60008061158e612246565b600060608060606000806115a0612271565b6115a8612225565b600099505b6008805490508a10156116085760088a8154811015156115c957fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff166002805490501015156115fd57611608565b8960010199506115ad565b6008805490508a10151561161f5760009a50611bbb565b60088a81548110151561162e57fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff168152505098506116948a611f2a565b60076000815480929190600101919050556001029750606060405190810160405280896000191681526020018a8152602001600480549050815250600560008a6000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff160217905550505060408201518160040155905050886040015160ff166040519080825280602002602001820160405280156117895781602001602082028038833980820191505090505b509650886040015160ff166040519080825280602002602001820160405280156117c25781602001602082028038833980820191505090505b509550886040015160ff166040519080825280602002602001820160405280156117fb5781602001602082028038833980820191505090505b509450600093505b886040015160ff16841015611a7c576002600081548110151561182257fe5b9060005260206000200154925060036000846000191660001916815260200190815260200160002060c060405190810160405290816000820154600019166000191681526020016001820160009054906101000a9004680100000000000000000267ffffffffffffffff191667ffffffffffffffff191681526020016001820160189054906101000a900461ffff1661ffff1661ffff16815260200160018201601a9054906101000a900461ffff1661ffff1661ffff16815260200160018201601c9054906101000a900461ffff1661ffff1661ffff1681526020016002820154815250509150604080519081016040528084600019168152602001836080015161ffff168152509050600481908060018154018082558091505090600182039060005260206000209060020201600090919290919091506000820151816000019060001916905560208201518160010160006101000a81548161ffff021916908361ffff160217905550505050876006836040015161ffff16846080015161ffff168560a0015101038154811015156119b857fe5b9060005260206000200181600019169055508287858151811015156119d957fe5b90602001906020020190600019169081600019168152505081602001518685815181101515611a0457fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff19168152505080602001518585815181101515611a3d57fe5b9060200190602002019061ffff16908161ffff1681525050611a5e83611fe7565b1515611a6f57611a6e60006120a5565b5b8380600101945050611803565b7f170aca2813372d685ae5df405903759eaec5d327852d4467bf2cdd312f9e006c888a6000015160008a8a8a60405180876000191660001916815260200186600019166000191681526020018560070b8152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b83811015611b18578082015181840152602081019050611afd565b50505050905001848103835286818151815260200191508051906020019060200280838360005b83811015611b5a578082015181840152602081019050611b3f565b50505050905001848103825285818151815260200191508051906020019060200280838360005b83811015611b9c578082015181840152602081019050611b81565b50505050905001995050505050505050505060405180910390a160019a505b5050505050505050505090565b611c45826001836040518082805190602001908083835b602083101515611c045780518252602082019150602081019050602083039250611bdf565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061211290919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fd211483f91fc6eff862467f8de606587a30c8fc9981056f051b897a418df803a826040518080602001828103825283818151815260200191508051906020019080838360005b83811015611cbe578082015181840152602081019050611ca3565b50505050905090810190601f168015611ceb5780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b611d79826001836040518082805190602001908083835b602083101515611d385780518252602082019150602081019050602083039250611d13565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061217090919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fbfec83d64eaa953f2708271a023ab9ee82057f8f3578d548c1a4ba0b5b700489826040518080602001828103825283818151815260200191508051906020019080838360005b83811015611df2578082015181840152602081019050611dd7565b50505050905090810190601f168015611e1f5780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff1614151515611e6c57600080fd5b8073ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3806000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b60016008805490500381141515611fcf576008600160088054905003815481101515611f5257fe5b9060005260206000209060030201600882815481101515611f6f57fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055509050505b60088054600190039081611fe391906122c2565b5050565b6000600360008360001916600019168152602001908152602001600020600101601c81819054906101000a900461ffff168092919060010191906101000a81548161ffff021916908361ffff16021790555050600360008360001916600019168152602001908152602001600020600101601a9054906101000a900461ffff1661ffff16600360008460001916600019168152602001908152602001600020600101601c9054906101000a900461ffff1661ffff1614159050919050565b600160028054905003811415156120fa5760026001600280549050038154811015156120cd57fe5b90600052602060002001546002828154811015156120e757fe5b9060005260206000200181600019169055505b6002805460019003908161210e91906121ce565b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b8154818355818111156121f5578183600052602060002091820191016121f491906122f4565b5b505050565b60a06040519081016040528060008019168152602001612218612319565b8152602001600081525090565b604080519081016040528060008019168152602001600061ffff1681525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b60c06040519081016040528060008019168152602001600067ffffffffffffffff19168152602001600061ffff168152602001600061ffff168152602001600061ffff168152602001600081525090565b8154818355818111156122ef576003028160030283600052602060002091820191016122ee9190612344565b5b505050565b61231691905b808211156123125760008160009055506001016122fa565b5090565b90565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b61238491905b808211156123805760008082016000905560018201600090556002820160006101000a81549060ff02191690555060030161234a565b5090565b905600a165627a7a723058205fb580b628770f8c13db2b84afe4219d7c8b295041074e1b283a7c66bad03d350029";

    public static final String FUNC_CHECKROLE = "checkRole";

    public static final String FUNC_ROLE_WHITELISTED = "ROLE_WHITELISTED";

    public static final String FUNC_HASROLE = "hasRole";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_REMOVEADDRESSESFROMWHITELIST = "removeAddressesFromWhitelist";

    public static final String FUNC_REMOVEADDRESSFROMWHITELIST = "removeAddressFromWhitelist";

    public static final String FUNC_GETSTATUS = "getStatus";

    public static final String FUNC_RENOUNCEOWNERSHIP = "renounceOwnership";

    public static final String FUNC_ADDADDRESSTOWHITELIST = "addAddressToWhitelist";

    public static final String FUNC_ADDNODE = "addNode";

    public static final String FUNC_OWNER = "owner";

    public static final String FUNC_WHITELIST = "whitelist";

    public static final String FUNC_GETCLUSTER = "getCluster";

    public static final String FUNC_ADDADDRESSESTOWHITELIST = "addAddressesToWhitelist";

    public static final String FUNC_TRANSFEROWNERSHIP = "transferOwnership";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<Int64>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes24>>() {}, new TypeReference<DynamicArray<Uint16>>() {}));
    ;

    public static final Event CODEENQUEUED_EVENT = new Event("CodeEnqueued", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event NEWNODE_EVENT = new Event("NewNode", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event ROLEADDED_EVENT = new Event("RoleAdded", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Utf8String>() {}));
    ;

    public static final Event ROLEREMOVED_EVENT = new Event("RoleRemoved", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Utf8String>() {}));
    ;

    public static final Event OWNERSHIPRENOUNCED_EVENT = new Event("OwnershipRenounced", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}));
    ;

    public static final Event OWNERSHIPTRANSFERRED_EVENT = new Event("OwnershipTransferred", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Address>(true) {}));
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

    public void checkRole(Address _operator, Utf8String _role) {
        throw new RuntimeException("cannot call constant function with void return type");
    }

    public RemoteCall<Utf8String> ROLE_WHITELISTED() {
        final Function function = new Function(FUNC_ROLE_WHITELISTED, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Utf8String>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Bool> hasRole(Address _operator, Utf8String _role) {
        final Function function = new Function(FUNC_HASROLE, 
                Arrays.<Type>asList(_operator, _role), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bool>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<TransactionReceipt> addCode(Bytes32 storageHash, Bytes32 storageReceipt, Uint8 clusterSize) {
        final Function function = new Function(
                FUNC_ADDCODE, 
                Arrays.<Type>asList(storageHash, storageReceipt, clusterSize), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> removeAddressesFromWhitelist(DynamicArray<Address> _operators) {
        final Function function = new Function(
                FUNC_REMOVEADDRESSESFROMWHITELIST, 
                Arrays.<Type>asList(_operators), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> removeAddressFromWhitelist(Address _operator) {
        final Function function = new Function(
                FUNC_REMOVEADDRESSFROMWHITELIST, 
                Arrays.<Type>asList(_operator), 
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

    public RemoteCall<TransactionReceipt> renounceOwnership() {
        final Function function = new Function(
                FUNC_RENOUNCEOWNERSHIP, 
                Arrays.<Type>asList(), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> addAddressToWhitelist(Address _operator) {
        final Function function = new Function(
                FUNC_ADDADDRESSTOWHITELIST, 
                Arrays.<Type>asList(_operator), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> addNode(Bytes32 nodeID, Bytes24 nodeAddress, Uint16 startPort, Uint16 endPort) {
        final Function function = new Function(
                FUNC_ADDNODE, 
                Arrays.<Type>asList(nodeID, nodeAddress, startPort, endPort), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Address> owner() {
        final Function function = new Function(FUNC_OWNER, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Address>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Bool> whitelist(Address _operator) {
        final Function function = new Function(FUNC_WHITELIST, 
                Arrays.<Type>asList(_operator), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bool>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Tuple5<Bytes32, Bytes32, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>> getCluster(Bytes32 clusterID) {
        final Function function = new Function(FUNC_GETCLUSTER, 
                Arrays.<Type>asList(clusterID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes24>>() {}, new TypeReference<DynamicArray<Uint16>>() {}));
        return new RemoteCall<Tuple5<Bytes32, Bytes32, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>>(
                new Callable<Tuple5<Bytes32, Bytes32, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>>() {
                    @Override
                    public Tuple5<Bytes32, Bytes32, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple5<Bytes32, Bytes32, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>(
                                (Bytes32) results.get(0), 
                                (Bytes32) results.get(1), 
                                (DynamicArray<Bytes32>) results.get(2), 
                                (DynamicArray<Bytes24>) results.get(3), 
                                (DynamicArray<Uint16>) results.get(4));
                    }
                });
    }

    public RemoteCall<TransactionReceipt> addAddressesToWhitelist(DynamicArray<Address> _operators) {
        final Function function = new Function(
                FUNC_ADDADDRESSESTOWHITELIST, 
                Arrays.<Type>asList(_operators), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> transferOwnership(Address _newOwner) {
        final Function function = new Function(
                FUNC_TRANSFEROWNERSHIP, 
                Arrays.<Type>asList(_newOwner), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public List<ClusterFormedEventResponse> getClusterFormedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CLUSTERFORMED_EVENT, transactionReceipt);
        ArrayList<ClusterFormedEventResponse> responses = new ArrayList<ClusterFormedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(0);
            typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(1);
            typedResponse.genesisTime = (Int64) eventValues.getNonIndexedValues().get(2);
            typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
            typedResponse.solverAddrs = (DynamicArray<Bytes24>) eventValues.getNonIndexedValues().get(4);
            typedResponse.solverPorts = (DynamicArray<Uint16>) eventValues.getNonIndexedValues().get(5);
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
                typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(1);
                typedResponse.genesisTime = (Int64) eventValues.getNonIndexedValues().get(2);
                typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
                typedResponse.solverAddrs = (DynamicArray<Bytes24>) eventValues.getNonIndexedValues().get(4);
                typedResponse.solverPorts = (DynamicArray<Uint16>) eventValues.getNonIndexedValues().get(5);
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

    public List<NewNodeEventResponse> getNewNodeEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(NEWNODE_EVENT, transactionReceipt);
        ArrayList<NewNodeEventResponse> responses = new ArrayList<NewNodeEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            NewNodeEventResponse typedResponse = new NewNodeEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<NewNodeEventResponse> newNodeEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, NewNodeEventResponse>() {
            @Override
            public NewNodeEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(NEWNODE_EVENT, log);
                NewNodeEventResponse typedResponse = new NewNodeEventResponse();
                typedResponse.log = log;
                typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<NewNodeEventResponse> newNodeEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(NEWNODE_EVENT));
        return newNodeEventObservable(filter);
    }

    public List<RoleAddedEventResponse> getRoleAddedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(ROLEADDED_EVENT, transactionReceipt);
        ArrayList<RoleAddedEventResponse> responses = new ArrayList<RoleAddedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            RoleAddedEventResponse typedResponse = new RoleAddedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<RoleAddedEventResponse> roleAddedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, RoleAddedEventResponse>() {
            @Override
            public RoleAddedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(ROLEADDED_EVENT, log);
                RoleAddedEventResponse typedResponse = new RoleAddedEventResponse();
                typedResponse.log = log;
                typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<RoleAddedEventResponse> roleAddedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(ROLEADDED_EVENT));
        return roleAddedEventObservable(filter);
    }

    public List<RoleRemovedEventResponse> getRoleRemovedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(ROLEREMOVED_EVENT, transactionReceipt);
        ArrayList<RoleRemovedEventResponse> responses = new ArrayList<RoleRemovedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            RoleRemovedEventResponse typedResponse = new RoleRemovedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<RoleRemovedEventResponse> roleRemovedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, RoleRemovedEventResponse>() {
            @Override
            public RoleRemovedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(ROLEREMOVED_EVENT, log);
                RoleRemovedEventResponse typedResponse = new RoleRemovedEventResponse();
                typedResponse.log = log;
                typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<RoleRemovedEventResponse> roleRemovedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(ROLEREMOVED_EVENT));
        return roleRemovedEventObservable(filter);
    }

    public List<OwnershipRenouncedEventResponse> getOwnershipRenouncedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(OWNERSHIPRENOUNCED_EVENT, transactionReceipt);
        ArrayList<OwnershipRenouncedEventResponse> responses = new ArrayList<OwnershipRenouncedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            OwnershipRenouncedEventResponse typedResponse = new OwnershipRenouncedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<OwnershipRenouncedEventResponse> ownershipRenouncedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, OwnershipRenouncedEventResponse>() {
            @Override
            public OwnershipRenouncedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(OWNERSHIPRENOUNCED_EVENT, log);
                OwnershipRenouncedEventResponse typedResponse = new OwnershipRenouncedEventResponse();
                typedResponse.log = log;
                typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<OwnershipRenouncedEventResponse> ownershipRenouncedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(OWNERSHIPRENOUNCED_EVENT));
        return ownershipRenouncedEventObservable(filter);
    }

    public List<OwnershipTransferredEventResponse> getOwnershipTransferredEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(OWNERSHIPTRANSFERRED_EVENT, transactionReceipt);
        ArrayList<OwnershipTransferredEventResponse> responses = new ArrayList<OwnershipTransferredEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            OwnershipTransferredEventResponse typedResponse = new OwnershipTransferredEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.newOwner = (Address) eventValues.getIndexedValues().get(1);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<OwnershipTransferredEventResponse> ownershipTransferredEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, OwnershipTransferredEventResponse>() {
            @Override
            public OwnershipTransferredEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(OWNERSHIPTRANSFERRED_EVENT, log);
                OwnershipTransferredEventResponse typedResponse = new OwnershipTransferredEventResponse();
                typedResponse.log = log;
                typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.newOwner = (Address) eventValues.getIndexedValues().get(1);
                return typedResponse;
            }
        });
    }

    public Observable<OwnershipTransferredEventResponse> ownershipTransferredEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(OWNERSHIPTRANSFERRED_EVENT));
        return ownershipTransferredEventObservable(filter);
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

        public Bytes32 storageHash;

        public Int64 genesisTime;

        public DynamicArray<Bytes32> solverIDs;

        public DynamicArray<Bytes24> solverAddrs;

        public DynamicArray<Uint16> solverPorts;
    }

    public static class CodeEnqueuedEventResponse {
        public Log log;

        public Bytes32 storageHash;
    }

    public static class NewNodeEventResponse {
        public Log log;

        public Bytes32 id;
    }

    public static class RoleAddedEventResponse {
        public Log log;

        public Address operator;

        public Utf8String role;
    }

    public static class RoleRemovedEventResponse {
        public Log log;

        public Address operator;

        public Utf8String role;
    }

    public static class OwnershipRenouncedEventResponse {
        public Log log;

        public Address previousOwner;
    }

    public static class OwnershipTransferredEventResponse {
        public Log log;

        public Address previousOwner;

        public Address newOwner;
    }
}
