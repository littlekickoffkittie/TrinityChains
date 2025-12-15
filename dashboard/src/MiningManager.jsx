import React, { useState, useEffect } from 'react';
import { Play, Pause, Settings, Zap, Award, TrendingUp, AlertCircle, CheckCircle, Loader, Copy } from 'lucide-react';

const MiningManager = ({ nodeUrl }) => {
  const [wallets, setWallets] = useState([]);
  const [selectedWalletName, setSelectedWalletName] = useState('');
  const [minerAddress, setMinerAddress] = useState('');
  const [isMining, setIsMining] = useState(false);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState({ type: '', text: '' });
  const [miningStats, setMiningStats] = useState({
    is_mining: false,
    blocks_mined: 0,
  });
  const [blockchainStats, setBlockchainStats] = useState(null);
  const [statsHistory, setStatsHistory] = useState([]);

  useEffect(() => {
    loadWallets();
  }, []);

  useEffect(() => {
    const interval = setInterval(fetchStats, 2000);
    fetchStats();
    return () => clearInterval(interval);
  }, [nodeUrl]);

  const loadWallets = () => {
    const stored = localStorage.getItem('trinity_wallets');
    const walletList = stored ? Object.keys(JSON.parse(stored)) : [];
    setWallets(walletList);
  };

  const getWalletAddress = (name) => {
    const stored = localStorage.getItem('trinity_wallets');
    const allWallets = stored ? JSON.parse(stored) : {};
    return allWallets[name]?.address || '';
  };

  const fetchStats = async () => {
    try {
      const [miningRes, blockchainRes] = await Promise.all([
        fetch(`${nodeUrl}/api/mining/status`, { credentials: 'include' }),
        fetch(`${nodeUrl}/api/blockchain/stats`, { credentials: 'include' })
      ]);

      if (miningRes.ok) {
        const miningData = await miningRes.json();
        setMiningStats(miningData);
      }

      if (blockchainRes.ok) {
        const blockchainData = await blockchainRes.json();
        setBlockchainStats(blockchainData);
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error);
    }
  };

  const startMining = async () => {
    const address = selectedWalletName ? getWalletAddress(selectedWalletName) : minerAddress;

    if (!address || address.trim() === '') {
      showMessage('error', 'Please select a wallet or enter a miner address');
      return;
    }

    setLoading(true);
    try {
      const response = await fetch(`${nodeUrl}/api/mining/start`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ miner_address: address }),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to start mining');
      }

      setIsMining(true);
      showMessage('success', 'Mining started!');
      fetchStats();
    } catch (error) {
      showMessage('error', error.message);
    } finally {
      setLoading(false);
    }
  };

  const stopMining = async () => {
    setLoading(true);
    try {
      const response = await fetch(`${nodeUrl}/api/mining/stop`, {
        method: 'POST',
        credentials: 'include',
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to stop mining');
      }

      setIsMining(false);
      showMessage('success', 'Mining stopped!');
      fetchStats();
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

  const copyToClipboard = (text) => {
    navigator.clipboard.writeText(text);
    showMessage('success', 'Copied to clipboard!');
  };

  const formatNumber = (num) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return num.toString();
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

      {/* Mining Status Card */}
      <div className={`rounded-lg p-6 border-2 transition-all ${
        miningStats.is_mining
          ? 'bg-gradient-to-br from-green-900/30 to-emerald-900/30 border-green-500 shadow-lg shadow-green-500/30'
          : 'bg-slate-900/50 border-purple-500/20'
      }`}>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            {miningStats.is_mining ? (
              <div className="bg-green-600/20 rounded-lg p-3">
                <Zap size={28} className="text-green-400 animate-pulse" />
              </div>
            ) : (
              <div className="bg-slate-700/50 rounded-lg p-3">
                <Zap size={28} className="text-slate-400" />
              </div>
            )}
            <div>
              <h3 className="text-2xl font-bold">{miningStats.is_mining ? 'Mining Active' : 'Mining Inactive'}</h3>
              <p className={`text-sm ${miningStats.is_mining ? 'text-green-300' : 'text-purple-300'}`}>
                {miningStats.is_mining ? 'Blocks are being mined' : 'Start mining to begin'}
              </p>
            </div>
          </div>
          <button
            onClick={miningStats.is_mining ? stopMining : startMining}
            disabled={loading}
            className={`flex items-center gap-2 px-6 py-3 rounded-lg font-semibold transition-all ${
              miningStats.is_mining
                ? 'bg-red-600 hover:bg-red-700 disabled:bg-slate-600'
                : 'bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700 disabled:from-slate-600 disabled:to-slate-600'
            }`}
          >
            {loading ? (
              <>
                <Loader size={20} className="animate-spin" />
                {miningStats.is_mining ? 'Stopping...' : 'Starting...'}
              </>
            ) : miningStats.is_mining ? (
              <>
                <Pause size={20} />
                Stop Mining
              </>
            ) : (
              <>
                <Play size={20} />
                Start Mining
              </>
            )}
          </button>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
            <p className="text-purple-300 text-sm mb-1">Blocks Mined</p>
            <p className="text-3xl font-bold">{miningStats.blocks_mined}</p>
          </div>
          <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
            <p className="text-purple-300 text-sm mb-1">Chain Height</p>
            <p className="text-3xl font-bold">{blockchainStats?.height || 0}</p>
          </div>
          <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
            <p className="text-purple-300 text-sm mb-1">Difficulty</p>
            <p className="text-3xl font-bold">{blockchainStats?.difficulty || 0}</p>
          </div>
          <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
            <p className="text-purple-300 text-sm mb-1">Mempool</p>
            <p className="text-3xl font-bold">{blockchainStats?.mempool_size || 0}</p>
          </div>
        </div>
      </div>

      {/* Miner Configuration */}
      <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20">
        <div className="flex items-center gap-3 mb-4">
          <div className="bg-purple-600/20 rounded-lg p-3">
            <Settings size={24} className="text-purple-400" />
          </div>
          <h3 className="text-xl font-bold">Mining Configuration</h3>
        </div>

        <div className="space-y-4">
          {/* Wallet Selection */}
          <div>
            <label className="text-sm text-purple-300 mb-2 block">Select Wallet (Optional)</label>
            <select
              value={selectedWalletName}
              onChange={(e) => {
                setSelectedWalletName(e.target.value);
                if (e.target.value) {
                  setMinerAddress(getWalletAddress(e.target.value));
                }
              }}
              disabled={miningStats.is_mining}
              className="w-full bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white focus:outline-none focus:border-purple-400 disabled:opacity-50"
            >
              <option value="">-- Use custom address --</option>
              {wallets.map(name => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
          </div>

          {/* Miner Address */}
          <div>
            <label className="text-sm text-purple-300 mb-2 block">Miner Address</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={minerAddress}
                onChange={(e) => setMinerAddress(e.target.value)}
                placeholder="Enter or select wallet address..."
                disabled={miningStats.is_mining || selectedWalletName !== ''}
                className="flex-1 bg-slate-800/50 border border-purple-500/30 rounded px-4 py-2 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400 disabled:opacity-50"
              />
              <button
                onClick={() => copyToClipboard(minerAddress)}
                className="p-2 bg-slate-700 hover:bg-slate-600 rounded transition-all"
                title="Copy address"
              >
                <Copy size={20} />
              </button>
            </div>
            <p className="text-purple-400 text-xs mt-2">
              {selectedWalletName ? `Using wallet: ${selectedWalletName}` : 'Enter a custom address or select a wallet'}
            </p>
          </div>

          {/* Info Box */}
          <div className="bg-blue-900/30 border border-blue-500/30 rounded p-4">
            <div className="flex gap-3">
              <AlertCircle className="text-blue-400 flex-shrink-0" size={20} />
              <div className="text-sm text-blue-200">
                <p className="font-semibold mb-1">Mining Rewards</p>
                <p>Block rewards will be sent to the miner address specified. Make sure to provide a valid address to receive rewards.</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Mining Performance */}
      {miningStats.is_mining && (
        <div className="bg-slate-900/50 rounded-lg p-6 border border-green-500/30 shadow-lg shadow-green-500/10">
          <div className="flex items-center gap-3 mb-4">
            <div className="bg-green-600/20 rounded-lg p-3">
              <TrendingUp size={24} className="text-green-400" />
            </div>
            <h3 className="text-xl font-bold">Mining Performance</h3>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-gradient-to-br from-green-600/20 to-emerald-600/20 rounded p-4 border border-green-500/30">
              <p className="text-green-300 text-sm mb-2">Blocks This Session</p>
              <p className="text-4xl font-bold text-green-400">{miningStats.blocks_mined}</p>
              <p className="text-green-400 text-xs mt-2">Keep mining to earn more rewards</p>
            </div>

            <div className="bg-gradient-to-br from-purple-600/20 to-pink-600/20 rounded p-4 border border-purple-500/30">
              <p className="text-purple-300 text-sm mb-2">Current Difficulty</p>
              <p className="text-4xl font-bold text-purple-400">{blockchainStats?.difficulty || 0}</p>
              <p className="text-purple-400 text-xs mt-2">Network difficulty level</p>
            </div>

            <div className="bg-gradient-to-br from-yellow-600/20 to-orange-600/20 rounded p-4 border border-yellow-500/30">
              <p className="text-yellow-300 text-sm mb-2">Pending Transactions</p>
              <p className="text-4xl font-bold text-yellow-400">{blockchainStats?.mempool_size || 0}</p>
              <p className="text-yellow-400 text-xs mt-2">In mempool, ready for mining</p>
            </div>
          </div>
        </div>
      )}

      {/* Start Mining Prompt */}
      {!miningStats.is_mining && (
        <div className="bg-gradient-to-br from-purple-900/30 to-pink-900/30 rounded-lg p-6 border border-purple-500/30">
          <div className="flex items-start gap-4">
            <div className="bg-purple-600/20 rounded-lg p-3 flex-shrink-0">
              <Award size={28} className="text-purple-400" />
            </div>
            <div>
              <h3 className="text-lg font-bold mb-2">Start Mining to Earn Rewards</h3>
              <p className="text-purple-300 text-sm mb-4">
                Configure your mining address and click "Start Mining" to begin earning block rewards. Each mined block will send rewards to your address.
              </p>
              <button
                onClick={startMining}
                disabled={loading || !minerAddress}
                className={`px-6 py-2 rounded-lg font-semibold transition-all flex items-center gap-2 ${
                  minerAddress
                    ? 'bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700'
                    : 'bg-slate-600 cursor-not-allowed opacity-50'
                }`}
              >
                <Play size={18} />
                Get Started with Mining
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default MiningManager;
