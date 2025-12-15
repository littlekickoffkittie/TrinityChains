import React, { useState, useEffect } from 'react';
import { Wallet, Plus, Copy, Download, Upload, Trash2, Eye, EyeOff, CheckCircle, AlertCircle } from 'lucide-react';

const WalletManager = ({ nodeUrl }) => {
  const [wallets, setWallets] = useState([]);
  const [selectedWallet, setSelectedWallet] = useState(null);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [showPrivateKey, setShowPrivateKey] = useState(false);
  const [newWalletName, setNewWalletName] = useState('');
  const [importName, setImportName] = useState('');
  const [importData, setImportData] = useState('');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState({ type: '', text: '' });
  const [balances, setBalances] = useState({});

  // Load wallets on mount
  useEffect(() => {
    loadWallets();
  }, []);

  // Fetch balance for selected wallet
  useEffect(() => {
    if (selectedWallet) {
      fetchBalance(selectedWallet.address);
    }
  }, [selectedWallet, nodeUrl]);

  const loadWallets = () => {
    const stored = localStorage.getItem('trinity_wallets');
    const walletList = stored ? Object.entries(JSON.parse(stored)) : [];
    setWallets(walletList.map(([name, data]) => ({ name, ...data })));
  };

  const fetchBalance = async (address) => {
    try {
      const response = await fetch(`${nodeUrl}/api/address/${address}/balance`, { credentials: 'include' });
      if (response.ok) {
        const data = await response.json();
        setBalances(prev => ({
          ...prev,
          [address]: parseFloat(data.balance || 0)
        }));
      }
    } catch (error) {
      console.error('Failed to fetch balance:', error);
    }
  };

  const createWallet = async () => {
    if (!newWalletName.trim()) {
      showMessage('error', 'Please enter a wallet name');
      return;
    }

    if (wallets.some(w => w.name === newWalletName)) {
      showMessage('error', 'Wallet name already exists');
      return;
    }

    setLoading(true);
    try {
      const response = await fetch(`${nodeUrl}/api/wallet/create`, {
        method: 'POST',
        credentials: 'include',
      });

      if (!response.ok) {
        throw new Error('Failed to create wallet');
      }

      const wallet = await response.json();
      
      // Save to localStorage
      const stored = localStorage.getItem('trinity_wallets') || '{}';
      const allWallets = JSON.parse(stored);
      allWallets[newWalletName] = wallet;
      localStorage.setItem('trinity_wallets', JSON.stringify(allWallets));

      loadWallets();
      setNewWalletName('');
      setShowCreateDialog(false);
      showMessage('success', 'Wallet created successfully!');
    } catch (error) {
      showMessage('error', error.message);
    } finally {
      setLoading(false);
    }
  };

  const importWallet = () => {
    if (!importName.trim()) {
      showMessage('error', 'Please enter a wallet name');
      return;
    }

    if (!importData.trim()) {
      showMessage('error', 'Please paste wallet data');
      return;
    }

    try {
      const parsed = JSON.parse(importData);
      const wallet = parsed[importName] || Object.values(parsed)[0];
      
      if (!wallet || !wallet.address) {
        throw new Error('Invalid wallet data');
      }

      const stored = localStorage.getItem('trinity_wallets') || '{}';
      const allWallets = JSON.parse(stored);
      allWallets[importName] = wallet;
      localStorage.setItem('trinity_wallets', JSON.stringify(allWallets));

      loadWallets();
      setImportName('');
      setImportData('');
      setShowImportDialog(false);
      showMessage('success', 'Wallet imported successfully!');
    } catch (error) {
      showMessage('error', `Import failed: ${error.message}`);
    }
  };

  const exportWallet = (wallet) => {
    try {
      const exportData = JSON.stringify({ [wallet.name]: { address: wallet.address, public_key: wallet.public_key } }, null, 2);
      const blob = new Blob([exportData], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${wallet.name}-wallet.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      showMessage('success', 'Wallet exported successfully!');
    } catch (error) {
      showMessage('error', 'Export failed');
    }
  };

  const deleteWallet = (walletName) => {
    if (confirm(`Are you sure you want to delete wallet "${walletName}"?`)) {
      const stored = localStorage.getItem('trinity_wallets') || '{}';
      const allWallets = JSON.parse(stored);
      delete allWallets[walletName];
      localStorage.setItem('trinity_wallets', JSON.stringify(allWallets));
      loadWallets();
      if (selectedWallet?.name === walletName) {
        setSelectedWallet(null);
      }
      showMessage('success', 'Wallet deleted');
    }
  };

  const copyToClipboard = (text, label) => {
    navigator.clipboard.writeText(text);
    showMessage('success', `${label} copied!`);
  };

  const showMessage = (type, text) => {
    setMessage({ type, text });
    setTimeout(() => setMessage({ type: '', text: '' }), 3000);
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

      {/* Wallet Actions */}
      <div className="flex gap-3">
        <button
          onClick={() => setShowCreateDialog(true)}
          className="flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 rounded-lg font-semibold transition-all"
        >
          <Plus size={20} />
          Create Wallet
        </button>
        <button
          onClick={() => setShowImportDialog(true)}
          className="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg font-semibold transition-all"
        >
          <Upload size={20} />
          Import Wallet
        </button>
      </div>

      {/* Create Dialog */}
      {showCreateDialog && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-slate-900 rounded-lg p-6 max-w-md w-full mx-4 border border-purple-500/30">
            <h3 className="text-xl font-bold mb-4">Create New Wallet</h3>
            <input
              type="text"
              value={newWalletName}
              onChange={(e) => setNewWalletName(e.target.value)}
              placeholder="Wallet name (e.g., 'MyWallet')"
              className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400 mb-4"
            />
            <div className="flex gap-3">
              <button
                onClick={createWallet}
                disabled={loading}
                className="flex-1 bg-purple-600 hover:bg-purple-700 disabled:bg-slate-600 px-4 py-2 rounded font-semibold transition-all"
              >
                {loading ? 'Creating...' : 'Create'}
              </button>
              <button
                onClick={() => setShowCreateDialog(false)}
                className="flex-1 bg-slate-700 hover:bg-slate-600 px-4 py-2 rounded font-semibold transition-all"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Import Dialog */}
      {showImportDialog && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-slate-900 rounded-lg p-6 max-w-md w-full mx-4 border border-purple-500/30">
            <h3 className="text-xl font-bold mb-4">Import Wallet</h3>
            <input
              type="text"
              value={importName}
              onChange={(e) => setImportName(e.target.value)}
              placeholder="Wallet name"
              className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400 mb-3"
            />
            <textarea
              value={importData}
              onChange={(e) => setImportData(e.target.value)}
              placeholder="Paste wallet JSON data"
              className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400 mb-4 h-32 font-mono text-sm"
            />
            <div className="flex gap-3">
              <button
                onClick={importWallet}
                className="flex-1 bg-purple-600 hover:bg-purple-700 px-4 py-2 rounded font-semibold transition-all"
              >
                Import
              </button>
              <button
                onClick={() => setShowImportDialog(false)}
                className="flex-1 bg-slate-700 hover:bg-slate-600 px-4 py-2 rounded font-semibold transition-all"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Wallet List */}
      <div className="space-y-4">
        {wallets.length === 0 ? (
          <div className="bg-slate-900/50 rounded-lg p-8 text-center border border-purple-500/20">
            <Wallet size={48} className="mx-auto text-purple-400 mb-4 opacity-50" />
            <p className="text-purple-300">No wallets yet. Create one to get started!</p>
          </div>
        ) : (
          wallets.map(wallet => (
            <div
              key={wallet.name}
              onClick={() => setSelectedWallet(wallet)}
              className={`bg-slate-900/50 rounded-lg p-6 border-2 cursor-pointer transition-all ${
                selectedWallet?.name === wallet.name
                  ? 'border-purple-500 shadow-lg shadow-purple-500/30'
                  : 'border-purple-500/20 hover:border-purple-500/40'
              }`}
            >
              <div className="flex items-start justify-between mb-4">
                <div>
                  <h3 className="text-lg font-bold">{wallet.name}</h3>
                  <p className="text-purple-300 text-sm">Balance: {balances[wallet.address]?.toFixed(2) || '0.00'} TRC</p>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      exportWallet(wallet);
                    }}
                    className="p-2 bg-slate-800 hover:bg-slate-700 rounded transition-all"
                    title="Export wallet"
                  >
                    <Download size={18} />
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteWallet(wallet.name);
                    }}
                    className="p-2 bg-red-900/30 hover:bg-red-900/50 rounded transition-all"
                    title="Delete wallet"
                  >
                    <Trash2 size={18} />
                  </button>
                </div>
              </div>

              {/* Address */}
              <div className="bg-slate-800/50 rounded p-3 mb-3">
                <p className="text-purple-300 text-xs mb-1">Address</p>
                <div className="flex items-center justify-between">
                  <p className="font-mono text-sm text-white break-all pr-2">{wallet.address}</p>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      copyToClipboard(wallet.address, 'Address');
                    }}
                    className="flex-shrink-0 p-2 hover:bg-slate-700 rounded transition-all"
                  >
                    <Copy size={16} />
                  </button>
                </div>
              </div>

              {/* Public Key */}
              {selectedWallet?.name === wallet.name && (
                <div className="bg-slate-800/50 rounded p-3 mb-3">
                  <div className="flex items-center justify-between mb-2">
                    <p className="text-purple-300 text-xs">Public Key</p>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setShowPrivateKey(!showPrivateKey);
                      }}
                      className="p-1 hover:bg-slate-700 rounded"
                    >
                      {showPrivateKey ? <EyeOff size={16} /> : <Eye size={16} />}
                    </button>
                  </div>
                  <div className="flex items-center justify-between">
                    <p className={`font-mono text-sm break-all pr-2 ${showPrivateKey ? 'text-white' : 'text-purple-400'}`}>
                      {showPrivateKey ? wallet.public_key : '••••••••••••••••••••••••••••••••'}
                    </p>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        copyToClipboard(wallet.public_key, 'Public Key');
                      }}
                      className="flex-shrink-0 p-2 hover:bg-slate-700 rounded transition-all"
                    >
                      <Copy size={16} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default WalletManager;
