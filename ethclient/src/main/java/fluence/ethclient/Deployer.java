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
    private static final String BINARY = "60806040526001600755336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506125d9806100586000396000f3006080604052600436106100e6576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630988ca8c146100eb57806318b919e914610174578063217fe6c6146102045780632238ba2f146102a557806324953eaa146102f1578063286dd3f5146103575780634e69d5601461039a578063715018a61461041a5780637b9417c8146104315780637ea29f62146104745780638da5cb5b146104d65780639b19251a1461052d578063c7c02e4414610588578063e2683e921461060e578063e2ec6ec314610742578063f2fde38b146107a8575b600080fd5b3480156100f757600080fd5b50610172600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f01602080910402602001604051908101604052809392919081815260200183838082843782019150505050505091929192905050506107eb565b005b34801561018057600080fd5b5061018961086c565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156101c95780820151818401526020810190506101ae565b50505050905090810190601f1680156101f65780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561021057600080fd5b5061028b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f01602080910402602001604051908101604052809392919081815260200183838082843782019150505050505091929192905050506108a5565b604051808215151515815260200191505060405180910390f35b3480156102b157600080fd5b506102ef60048036038101908080356000191690602001909291908035600019169060200190929190803560ff16906020019092919050505061092c565b005b3480156102fd57600080fd5b5061035560048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050610a2c565b005b34801561036357600080fd5b50610398600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610ac8565b005b3480156103a657600080fd5b506103af610b65565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156104045780820151818401526020810190506103e9565b5050505090500194505050505060405180910390f35b34801561042657600080fd5b5061042f610c29565b005b34801561043d57600080fd5b50610472600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610d2b565b005b34801561048057600080fd5b506104d46004803603810190808035600019169060200190929190803567ffffffffffffffff19169060200190929190803561ffff169060200190929190803561ffff169060200190929190505050610dc8565b005b3480156104e257600080fd5b506104eb6110c7565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561053957600080fd5b5061056e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506110ec565b604051808215151515815260200191505060405180910390f35b34801561059457600080fd5b506105b76004803603810190808035600019169060200190929190505050611134565b6040518080602001828103825283818151815260200191508051906020019060200280838360005b838110156105fa5780820151818401526020810190506105df565b505050509050019250505060405180910390f35b34801561061a57600080fd5b5061063d60048036038101908080356000191690602001909291905050506112c8565b6040518086600019166000191681526020018560001916600019168152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b838110156106a4578082015181840152602081019050610689565b50505050905001848103835286818151815260200191508051906020019060200280838360005b838110156106e65780820151818401526020810190506106cb565b50505050905001848103825285818151815260200191508051906020019060200280838360005b8381101561072857808201518184015260208101905061070d565b505050509050019850505050505050505060405180910390f35b34801561074e57600080fd5b506107a660048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050611633565b005b3480156107b457600080fd5b506107e9600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506116cf565b005b610868826001836040518082805190602001908083835b6020831015156108275780518252602082019150602081019050602083039250610802565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061173690919063ffffffff16565b5050565b6040805190810160405280600981526020017f77686974656c697374000000000000000000000000000000000000000000000081525081565b6000610924836001846040518082805190602001908083835b6020831015156108e357805182526020820191506020810190506020830392506108be565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061174f90919063ffffffff16565b905092915050565b610935336110ec565b151561094057610a27565b600860606040519081016040528085600019168152602001846000191681526020018360ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff1602179055505050506109e06117a8565b1515610a26577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8360405180826000191660001916815260200191505060405180910390a15b5b505050565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610a8957600080fd5b600090505b8151811015610ac457610ab78282815181101515610aa857fe5b90602001906020020151610ac8565b8080600101915050610a8e565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610b2357600080fd5b610b62816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611dee565b50565b600080606080600080600880549050604051908082528060200260200182016040528015610ba25781602001602082028038833980820191505090505b509250600091505b600880549050821015610c0f57600882815481101515610bc657fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff168383815181101515610bf657fe5b9060200190602002018181525050816001019150610baa565b606590508060028054905084955095509550505050909192565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610c8457600080fd5b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167ff8df31144d9c2f0f6b59d69b8b98abd5459d07f2742c4df920b25aae33c6482060405160405180910390a260008060006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610d8657600080fd5b610dc5816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611f22565b50565b610dd1336110ec565b1515610ddc576110c1565b60006001026003600086600019166000191681526020019081526020016000206000015460001916141515610e79576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601f8152602001807f54686973206e6f646520697320616c726561647920726567697374657265640081525060200191505060405180910390fd5b8061ffff168261ffff16101515610ef8576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260208152602001807f506f72742072616e676520697320656d707479206f7220696e636f727265637481525060200191505060405180910390fd5b60c060405190810160405280856000191681526020018467ffffffffffffffff191681526020018361ffff1681526020018261ffff1681526020018361ffff1681526020016006805490508152506003600086600019166000191681526020019081526020016000206000820151816000019060001916905560208201518160010160006101000a81548177ffffffffffffffffffffffffffffffffffffffffffffffff0219169083680100000000000000009004021790555060408201518160010160186101000a81548161ffff021916908361ffff160217905550606082015181600101601a6101000a81548161ffff021916908361ffff160217905550608082015181600101601c6101000a81548161ffff021916908361ffff16021790555060a08201518160020155905050600284908060018154018082558091505090600182039060005260206000200160009091929091909150906000191690555081810361ffff1660068181805490500191508161107791906123f4565b507fb0cd47a7093fb93a9ce97304d3afb8df43e02e48502e47fd5fbb6c4020d935b58460405180826000191660001916815260200191505060405180910390a16110bf6117a8565b505b50505050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600061112d826040805190810160405280600981526020017f77686974656c69737400000000000000000000000000000000000000000000008152506108a5565b9050919050565b606061113e612420565b6060600060036000866000191660001916815260200190815260200160002060c060405190810160405290816000820154600019166000191681526020016001820160009054906101000a9004680100000000000000000267ffffffffffffffff191667ffffffffffffffff191681526020016001820160189054906101000a900461ffff1661ffff1661ffff16815260200160018201601a9054906101000a900461ffff1661ffff1661ffff16815260200160018201601c9054906101000a900461ffff1661ffff1661ffff1681526020016002820154815250509250826040015183608001510361ffff166040519080825280602002602001820160405280156112595781602001602082028038833980820191505090505b509150600090505b81518110156112bd576006818460a001510181548110151561127f57fe5b9060005260206000200154828281518110151561129857fe5b9060200190602002019060001916908160001916815250508080600101915050611261565b819350505050919050565b60008060608060606112d8612471565b606080606060006112e761249c565b600560008d600019166000191681526020019081526020016000206060604051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff1681525050815260200160048201548152505095506000600102866000015160001916111515611405576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b85602001516040015160ff1660405190808252806020026020018201604052801561143f5781602001602082028038833980820191505090505b50945085602001516040015160ff1660405190808252806020026020018201604052801561147c5781602001602082028038833980820191505090505b50935085602001516040015160ff166040519080825280602002602001820160405280156114b95781602001602082028038833980820191505090505b509250600091505b85602001516040015160ff16821015611605576004828760400151018154811015156114e957fe5b90600052602060002090600202016040805190810160405290816000820154600019166000191681526020016001820160009054906101000a900461ffff1661ffff1661ffff168152505090508060000151858381518110151561154957fe5b9060200190602002019060001916908160001916815250506003600082600001516000191660001916815260200190815260200160002060010160009054906101000a9004680100000000000000000284838151811015156115a757fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff191681525050806020015183838151811015156115e057fe5b9060200190602002019061ffff16908161ffff168152505081806001019250506114c1565b8560200151600001518660200151602001518686869a509a509a509a509a5050505050505091939590929450565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561169057600080fd5b600090505b81518110156116cb576116be82828151811015156116af57fe5b90602001906020020151610d2b565b8080600101915050611695565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561172a57600080fd5b61173381612056565b50565b611740828261174f565b151561174b57600080fd5b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060009054906101000a900460ff16905092915050565b6000806117b36124bd565b600060608060606000806117c5612420565b6117cd61249c565b6000809a505b6008805490508b101561182e5760088b8154811015156117ef57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff166002805490501015156118235761182e565b8a6001019a506117d3565b6008805490508b1015156118455760009b50611de0565b60088b81548110151561185457fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff168152505099506118ba8b612150565b600760008154809291906001019190505560010298506060604051908101604052808a6000191681526020018b8152602001600480549050815250600560008b6000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff160217905550505060408201518160040155905050896040015160ff166040519080825280602002602001820160405280156119af5781602001602082028038833980820191505090505b509750896040015160ff166040519080825280602002602001820160405280156119e85781602001602082028038833980820191505090505b509650896040015160ff16604051908082528060200260200182016040528015611a215781602001602082028038833980820191505090505b509550600094505b896040015160ff16851015611ca25760026000815481101515611a4857fe5b9060005260206000200154935060036000856000191660001916815260200190815260200160002060c060405190810160405290816000820154600019166000191681526020016001820160009054906101000a9004680100000000000000000267ffffffffffffffff191667ffffffffffffffff191681526020016001820160189054906101000a900461ffff1661ffff1661ffff16815260200160018201601a9054906101000a900461ffff1661ffff1661ffff16815260200160018201601c9054906101000a900461ffff1661ffff1661ffff1681526020016002820154815250509250604080519081016040528085600019168152602001846080015161ffff168152509150600482908060018154018082558091505090600182039060005260206000209060020201600090919290919091506000820151816000019060001916905560208201518160010160006101000a81548161ffff021916908361ffff160217905550505050886006846040015161ffff16856080015161ffff168660a001510103815481101515611bde57fe5b906000526020600020018160001916905550838886815181101515611bff57fe5b90602001906020020190600019169081600019168152505082602001518786815181101515611c2a57fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff19168152505081602001518686815181101515611c6357fe5b9060200190602002019061ffff16908161ffff1681525050611c848461220d565b1515611c9557611c9460006122cb565b5b8480600101955050611a29565b4290507f28c3d361196410d2059b40d53bf75ae21adebcec217c5a2564746ed2c3427fd2898b60000151838b8b8b6040518087600019166000191681526020018660001916600019168152602001858152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b83811015611d3d578082015181840152602081019050611d22565b50505050905001848103835286818151815260200191508051906020019060200280838360005b83811015611d7f578082015181840152602081019050611d64565b50505050905001848103825285818151815260200191508051906020019060200280838360005b83811015611dc1578082015181840152602081019050611da6565b50505050905001995050505050505050505060405180910390a160019b505b505050505050505050505090565b611e6b826001836040518082805190602001908083835b602083101515611e2a5780518252602082019150602081019050602083039250611e05565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061233890919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fd211483f91fc6eff862467f8de606587a30c8fc9981056f051b897a418df803a826040518080602001828103825283818151815260200191508051906020019080838360005b83811015611ee4578082015181840152602081019050611ec9565b50505050905090810190601f168015611f115780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b611f9f826001836040518082805190602001908083835b602083101515611f5e5780518252602082019150602081019050602083039250611f39565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061239690919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fbfec83d64eaa953f2708271a023ab9ee82057f8f3578d548c1a4ba0b5b700489826040518080602001828103825283818151815260200191508051906020019080838360005b83811015612018578082015181840152602081019050611ffd565b50505050905090810190601f1680156120455780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff161415151561209257600080fd5b8073ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3806000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b600160088054905003811415156121f557600860016008805490500381548110151561217857fe5b906000526020600020906003020160088281548110151561219557fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055509050505b6008805460019003908161220991906124e8565b5050565b6000600360008360001916600019168152602001908152602001600020600101601c81819054906101000a900461ffff168092919060010191906101000a81548161ffff021916908361ffff16021790555050600360008360001916600019168152602001908152602001600020600101601a9054906101000a900461ffff1661ffff16600360008460001916600019168152602001908152602001600020600101601c9054906101000a900461ffff1661ffff1614159050919050565b600160028054905003811415156123205760026001600280549050038154811015156122f357fe5b906000526020600020015460028281548110151561230d57fe5b9060005260206000200181600019169055505b6002805460019003908161233491906123f4565b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b81548183558181111561241b5781836000526020600020918201910161241a919061251a565b5b505050565b60c06040519081016040528060008019168152602001600067ffffffffffffffff19168152602001600061ffff168152602001600061ffff168152602001600061ffff168152602001600081525090565b60a0604051908101604052806000801916815260200161248f61253f565b8152602001600081525090565b604080519081016040528060008019168152602001600061ffff1681525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b81548183558181111561251557600302816003028360005260206000209182019101612514919061256a565b5b505050565b61253c91905b80821115612538576000816000905550600101612520565b5090565b90565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b6125aa91905b808211156125a65760008082016000905560018201600090556002820160006101000a81549060ff021916905550600301612570565b5090565b905600a165627a7a72305820a1c89e65cc7c96c7377dbc4ca3c7e3fef3d05876f4666e4dba26d389db037eb90029";

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

    public static final String FUNC_GETNODECLUSTERS = "getNodeClusters";

    public static final String FUNC_GETCLUSTER = "getCluster";

    public static final String FUNC_ADDADDRESSESTOWHITELIST = "addAddressesToWhitelist";

    public static final String FUNC_TRANSFEROWNERSHIP = "transferOwnership";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<Uint256>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes24>>() {}, new TypeReference<DynamicArray<Uint16>>() {}));
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

    public RemoteCall<DynamicArray<Bytes32>> getNodeClusters(Bytes32 nodeID) {
        final Function function = new Function(FUNC_GETNODECLUSTERS, 
                Arrays.<Type>asList(nodeID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<DynamicArray<Bytes32>>() {}));
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
            typedResponse.genesisTime = (Uint256) eventValues.getNonIndexedValues().get(2);
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
                typedResponse.genesisTime = (Uint256) eventValues.getNonIndexedValues().get(2);
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

        public Uint256 genesisTime;

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
