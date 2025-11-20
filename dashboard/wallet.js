// Wallet functionality
let walletAddress = null;
let secretKey = null;

// Create a new wallet
async function createWallet() {
    try {
        const response = await fetch(`${API_BASE}/wallet/create`, {
            method: 'POST'
        });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const wallet = await response.json();
        walletAddress = wallet.address;
        secretKey = wallet.secret_key;
        updateWalletUI();
    } catch (error) {
        console.error('[createWallet] Error:', error);
    }
}

// Load a wallet from JSON
function loadWallet(json) {
    try {
        const wallet = JSON.parse(json);
        walletAddress = wallet.address;
        secretKey = wallet.secret_key;
        updateWalletUI();
        document.getElementById('loadWalletModal').style.display = 'none';
    } catch (error) {
        console.error('[loadWallet] Error:', error);
        alert('Invalid wallet JSON.');
    }
}

// Update the wallet UI
function updateWalletUI() {
    if (walletAddress) {
        document.getElementById('walletAddress').textContent = formatHash(walletAddress);
        calculateBalance();
    }
}

// Calculate the wallet balance
async function calculateBalance() {
    if (!walletAddress) {
        return;
    }

    try {
        const response = await fetch(`${API_BASE}/blockchain/utxos/${walletAddress}`);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const utxos = await response.json();
        const balance = utxos.length;
        document.getElementById('walletBalance').textContent = balance;
    } catch (error) {
        console.error('[calculateBalance] Error:', error);
    }
}

// Calculate the hash of a transaction
function hashTransaction(tx) {
    const txString = `${tx.from_address}${tx.to_address}${tx.amount}`;
    return CryptoJS.SHA256(txString).toString();
}

// Sign a transaction
function signTransaction(tx, secretKey) {
    const ec = new elliptic.ec('secp256k1');
    const key = ec.keyFromPrivate(secretKey, 'hex');
    const txHash = hashTransaction(tx);
    const signature = key.sign(txHash);
    return signature.toDER('hex');
}

// Send a transaction
async function sendTransaction() {
    if (!walletAddress || !secretKey) {
        alert('Please create or load a wallet first.');
        return;
    }

    const toAddress = prompt('Enter the recipient address:');
    if (!toAddress) {
        return;
    }

    const amount = prompt('Enter the amount to send:');
    if (!amount) {
        return;
    }

    try {
        // Create the transaction
        const tx = {
            from_address: walletAddress,
            to_address: toAddress,
            amount: parseInt(amount)
        };

        // Sign the transaction
        const signature = signTransaction(tx, secretKey);

        // Send the transaction to the API
        const response = await fetch(`${API_BASE}/wallet/send`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                transaction: tx,
                signature: signature
            })
        });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const txHash = await response.text();
        alert(`Transaction sent! Hash: ${txHash}`);
    } catch (error) {
        console.error('[sendTransaction] Error:', error);
    }
}
