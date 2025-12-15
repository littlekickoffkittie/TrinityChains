import React, { useState, useEffect } from 'react';
import { Send, History, DollarSign, ArrowUp, ArrowDown, Clock, CheckCircle, AlertCircle, Loader } from 'lucide-react';

const TransactionManager = ({ nodeUrl }) => {
  const [wallets, setWallets] = useState([]);
  const [selectedWalletName, setSelectedWalletName] = useState('');
  const [toAddress, setToAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [fee, setFee] = useState('1.0');
  const [memo, setMemo] = useState('');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState({ type: '', text: '' });
  const [transactions, setTransactions] = useState([]);
  const [balance, setBalance] = useState(0);
  const [mempool, setMempool] = useState([]);
  const [activeTab, setActiveTab] = useState('send');

  useEffect(() => {
    loadWallets();
  }, []);

  useEffect(() => {
    if (selectedWalletName) {
      const wallet = getWallet(selectedWalletName);
      if (wallet) {
        fetchBalance(wallet.address);
        fetchTransactionHistory(wallet.address);
      }
    }
  }, [selectedWalletName, nodeUrl]);

  useEffect(() => {
    const interval = setInterval(fetchMempool, 5000);
    fetchMempool();
    return () => clearInterval(interval);
  }, [nodeUrl]);

  const loadWallets = () => {
    const stored = localStorage.getItem('trinity_wallets');
    const walletList = stored ? Object.keys(JSON.parse(stored)) : [];
    setWallets(walletList);
    if (walletList.length > 0 && !selectedWalletName) {
      setSelectedWalletName(walletList[0]);
    }
  };

  const getWallet = (name) => {
    const stored = localStorage.getItem('trinity_wallets');
    const allWallets = stored ? JSON.parse(stored) : {};
    return allWallets[name] || null;
  };

  const fetchBalance = async (address) => {
    try {
      const response = await fetch(`${nodeUrl}/api/address/${address}/balance`, { credentials: 'include' });
      if (response.ok) {
        const data = await response.json();
        setBalance(parseFloat(data.balance || 0));
      }
    } catch (error) {
      console.error('Failed to fetch balance:', error);
    }
  };

  const fetchTransactionHistory = async (address) => {
    try {
      const response = await fetch(`${nodeUrl}/api/address/${address}/transactions`, { credentials: 'include' });
      if (response.ok) {
        const data = await response.json();
        setTransactions(data.transactions || []);
      }
    } catch (error) {
      console.error('Failed to fetch transaction history:', error);
    }
  };

  const fetchMempool = async () => {
    try {
      const response = await fetch(`${nodeUrl}/api/mempool`, { credentials: 'include' });
      if (response.ok) {
        const data = await response.json();
        setMempool(data.transactions || []);
      }
    } catch (error) {
      console.error('Failed to fetch mempool:', error);
    }
  };

  const sendTransaction = async () => {
    if (!selectedWalletName) {
      showMessage('error', 'Please select a wallet');
      return;
    }

    if (!toAddress.trim()) {
      showMessage('error', 'Please enter recipient address');
      return;
    }

    if (!amount || parseFloat(amount) <= 0) {
      showMessage('error', 'Please enter a valid amount');
      return;
    }

    if (parseFloat(amount) > balance) {
      showMessage('error', 'Insufficient balance');
      return;
    }

    setLoading(true);
    try {
      const wallet = getWallet(selectedWalletName);
      
      // Create transaction object
      const transaction = {
        Transfer: {
          input_hash: '0x' + '0'.repeat(64), // Placeholder
          new_owner: toAddress,
          sender: wallet.address,
          amount: amount,
          fee_area: fee,
          nonce: 0,
          signature: '0x' + '0'.repeat(128), // Placeholder - in real app, sign with private key
          public_key: wallet.public_key,
        }
      };

      const response = await fetch(`${nodeUrl}/api/transaction`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(transaction),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Transaction failed');
      }

      showMessage('success', 'Transaction submitted successfully!');
      setToAddress('');
      setAmount('');
      setFee('1.0');
      setMemo('');
      
      // Refresh data
      if (wallet) {
        fetchBalance(wallet.address);
        fetchTransactionHistory(wallet.address);
      }
      fetchMempool();
    } catch (error) {
      showMessage('error', error.message);
    } finally {
      setLoading(false);
    }
  };

  const showMessage = (type, text) => {
    setMessage({ type, text });
    setTimeout(() => setMessage({ type: '', text: '' }), 3000);
  };

  const formatAddress = (addr) => {
    if (!addr) return 'N/A';
    return `${addr.slice(0, 12)}...${addr.slice(-10)}`;
  };

  return (
    <div className="space-y-6">
      {/* Message Display */}
      {message.text && (
        <div className={`flex items-center gap-3 p-4 rounded-lg border-l-4 ${
          message.type === 'success' 
            ? 'bg-green-900/30 border-green-500 text-green-200'
            : 'bg-red-900/30 border-red-500 text-red-200'
        }`}>
          {message.type === 'success' ? <CheckCircle size={20} /> : <AlertCircle size={20} />}
          {message.text}
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-2 border-b border-purple-500/20">
        <button
          onClick={() => setActiveTab('send')}
          className={`flex items-center gap-2 px-6 py-3 font-semibold transition-all border-b-2 ${
            activeTab === 'send'
              ? 'border-purple-500 text-purple-300'
              : 'border-transparent text-purple-400 hover:text-purple-300'
          }`}
        >
          <Send size={18} />
          Send
        </button>
        <button
          onClick={() => setActiveTab('history')}
          className={`flex items-center gap-2 px-6 py-3 font-semibold transition-all border-b-2 ${
            activeTab === 'history'
              ? 'border-purple-500 text-purple-300'
              : 'border-transparent text-purple-400 hover:text-purple-300'
          }`}
        >
          <History size={18} />
          History
        </button>
        <button
          onClick={() => setActiveTab('mempool')}
          className={`flex items-center gap-2 px-6 py-3 font-semibold transition-all border-b-2 ${
            activeTab === 'mempool'
              ? 'border-purple-500 text-purple-300'
              : 'border-transparent text-purple-400 hover:text-purple-300'
          }`}
        >
          <Loader size={18} />
          Mempool
        </button>
      </div>

      {/* Send Tab */}
      {activeTab === 'send' && (
        <div className="space-y-6">
          {/* Wallet Selection & Balance */}
          <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20">
            <label className="text-sm text-purple-300 mb-2 block">Select Wallet</label>
            {wallets.length === 0 ? (
              <div className="bg-slate-800/50 rounded p-4 text-center text-purple-300">
                No wallets found. Create one first!
              </div>
            ) : (
              <>
                <select
                  value={selectedWalletName}
                  onChange={(e) => setSelectedWalletName(e.target.value)}
                  className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white focus:outline-none focus:border-purple-400 mb-4"
                >
                  {wallets.map(name => (
                    <option key={name} value={name}>{name}</option>
                  ))}
                </select>
                <div className="bg-gradient-to-r from-purple-600/20 to-pink-600/20 rounded p-4 border border-purple-500/30">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-purple-300 text-sm">Available Balance</p>
                      <p className="text-3xl font-bold">{balance.toFixed(2)}</p>
                    </div>
                    <DollarSign className="text-purple-400" size={40} />
                  </div>
                </div>
              </>
            )}
          </div>

          {/* Send Form */}
          {wallets.length > 0 && (
            <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20 space-y-4">
              <div>
                <label className="text-sm text-purple-300 mb-2 block">Recipient Address</label>
                <input
                  type="text"
                  value={toAddress}
                  onChange={(e) => setToAddress(e.target.value)}
                  placeholder="Enter recipient address..."
                  className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-sm text-purple-300 mb-2 block">Amount (TRC)</label>
                  <input
                    type="number"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0.00"
                    step="0.01"
                    min="0"
                    className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400"
                  />
                </div>
                <div>
                  <label className="text-sm text-purple-300 mb-2 block">Fee (TRC)</label>
                  <input
                    type="number"
                    value={fee}
                    onChange={(e) => setFee(e.target.value)}
                    placeholder="1.00"
                    step="0.01"
                    min="0"
                    className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400"
                  />
                </div>
              </div>

              <div>
                <label className="text-sm text-purple-300 mb-2 block">Memo (Optional)</label>
                <textarea
                  value={memo}
                  onChange={(e) => setMemo(e.target.value)}
                  placeholder="Add a note to this transaction..."
                  className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400 h-20"
                />
              </div>

              <div className="bg-slate-800/30 rounded p-4 border border-purple-500/10">
                <p className="text-purple-300 text-sm mb-2">Transaction Summary</p>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-purple-400">Amount:</span>
                    <span className="font-mono">{amount || '0.00'} TRC</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-purple-400">Fee:</span>
                    <span className="font-mono">{fee || '0.00'} TRC</span>
                  </div>
                  <div className="flex justify-between border-t border-purple-500/20 pt-2 font-bold">
                    <span className="text-purple-300">Total:</span>
                    <span className="font-mono">{(parseFloat(amount || 0) + parseFloat(fee || 0)).toFixed(2)} TRC</span>
                  </div>
                </div>
              </div>

              <button
                onClick={sendTransaction}
                disabled={loading || !selectedWalletName}
                className="w-full bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 disabled:from-slate-600 disabled:to-slate-600 px-6 py-3 rounded-lg font-semibold transition-all flex items-center justify-center gap-2"
              >
                {loading ? (
                  <>
                    <Loader size={20} className="animate-spin" />
                    Sending...
                  </>
                ) : (
                  <>
                    <Send size={20} />
                    Send Transaction
                  </>
                )}
              </button>
            </div>
          )}
        </div>
      )}

      {/* History Tab */}
      {activeTab === 'history' && (
        <div className="space-y-4">
          {transactions.length === 0 ? (
            <div className="bg-slate-900/50 rounded-lg p-8 text-center border border-purple-500/20">
              <History size={48} className="mx-auto text-purple-400 mb-4 opacity-50" />
              <p className="text-purple-300">No transactions yet</p>
            </div>
          ) : (
            transactions.map((tx, idx) => (
              <div key={idx} className="bg-slate-900/50 rounded-lg p-4 border border-purple-500/20 hover:border-purple-500/40 transition-all">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="bg-purple-600/20 rounded-lg p-3">
                      <ArrowUp size={24} className="text-purple-400" />
                    </div>
                    <div>
                      <p className="text-white font-semibold">Transaction</p>
                      <p className="text-purple-400 text-sm font-mono">{tx.hash?.slice(0, 20) || 'N/A'}...</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-green-400 font-bold">{tx.amount || '0'} TRC</p>
                    <p className="text-purple-400 text-sm flex items-center justify-end gap-1">
                      <Clock size={14} />
                      {new Date(tx.timestamp).toLocaleDateString()}
                    </p>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {/* Mempool Tab */}
      {activeTab === 'mempool' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-bold">Pending Transactions</h3>
            <span className="bg-purple-600/20 px-3 py-1 rounded-full text-sm text-purple-300">{mempool.length} transactions</span>
          </div>
          {mempool.length === 0 ? (
            <div className="bg-slate-900/50 rounded-lg p-8 text-center border border-purple-500/20">
              <CheckCircle size={48} className="mx-auto text-green-400 mb-4 opacity-50" />
              <p className="text-purple-300">No pending transactions</p>
            </div>
          ) : (
            mempool.map((tx, idx) => (
              <div key={idx} className="bg-slate-900/50 rounded-lg p-4 border border-purple-500/20 hover:border-purple-500/40 transition-all">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="bg-yellow-600/20 rounded-lg p-3">
                      <Loader size={24} className="text-yellow-400 animate-spin" />
                    </div>
                    <div>
                      <p className="text-white font-semibold">Pending</p>
                      <p className="text-purple-400 text-sm font-mono">{formatAddress(tx.hash || '')}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-yellow-400 font-bold">{tx.amount || '0'} TRC</p>
                    <p className="text-purple-400 text-sm">In mempool</p>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};

export default TransactionManager;
