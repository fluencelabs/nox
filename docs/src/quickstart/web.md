# Frontend
For this part, you will need installed `npm`. Please refer to [npm docs](https://www.npmjs.com/get-npm) for installation instructions.

## Preparing web app
Let's clone a simple web app template:
```bash
~ $ git clone https://github.com/fluencelabs/frontend-template
~ $ cd frontend-template
~/frontend-template $ 
```

There are just three files (except for README, LICENSE and .gitignore):
- `package.json` that declares needed dependencies
- `webpack.config.js` needed for the webpack to work
- `index.js` that imports `fluence` js library and shows how to connect to a cluster

`fluence` js library is an SDK, that can take cluster entry points from Ethereum smart contract with `connect` method and invoke commands to a cluster by method `invoke`.

Let's take a look at `index.js`:
```javascript

// address of the Fluence smart contract on Ethereum.
let contractAddress = "0x074a79f29c613f4f7035cec582d0f7e4d3cda2e7";

// Address of the Ethereum node. If set to `undefined`, MetaMask will be used to send transactions.
let ethUrl = "http://207.154.240.52:8545/";

// appId of the backend as seen in Fluence smart contract.
let appId = "6";
...
// create a session between client and backend application
fluence.connect(contractAddress, appId, ethUrl).then((s) => {
  console.log("Session created");
  window.session = s;
  helloBtn.disabled = false;
});
...
// set callback on button click
helloBtn.addEventListener("click", send)

// send username as a transaction and display results in grettingLbl
function send() {
  const username = usernameInput.value.trim();
  let result = session.invoke(username);
  getResultString(result).then(function (str) {
    greetingLbl.innerHTML = str;
  });
}
```

This code creates a session, and saves it to `window.session`, so it can be used later. 

Then, it assigns `send()` function on the button, which will call the `invoke` method. `invoke` takes a string from an input, and sends it to the backend as a transaction. The result will be displayed in a greeting label.

## Running and using
Please make sure you have changed `appId` to the appId of existing backend. If you haven't published your backend yet, please take a look at [Publishing guide](publish.md). You can also use backends published by other people, take a look at 

To install all dependencies, compile and run the application, run in the terminal:
```bash
~/frontend-template $ npm install
~/frontend-template $ npm run start
> frontend-template@1.0.0 start /private/tmp/frontend-template
> webpack-dev-server

ℹ ｢wds｣: Project is running at http://localhost:8080/
...
```

Now you can open http://localhost:8080/ in your browser. You will see an input text box and a disabled button. Button will become enabled once AppSession is created. You can enter your name, and press `Say hello!` button, and greeting will be displayed next to `Result:`.

You can also open Developer Console, and you'll see a log about session creation:
```
...
Connecting web3 to http://207.154.232.92:8545
...
Session created
```

<div style="text-align:center">
<kbd>
<img src="../images/helloworld.png" width="529px"/>
</kbd>
<br><br><br>
</div>

You can also use Fluence from within Developer console as follows:
```javascript
let result = session.invoke("myName");
<undefined>
logResultAsString(result);
<undefined>
Hello, world! From user myName
```
