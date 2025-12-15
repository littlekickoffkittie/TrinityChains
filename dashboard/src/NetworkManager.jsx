import React, { useState, useEffect } from 'react';
import { Network, Server, Globe, Zap, Activity, TrendingUp, AlertCircle, CheckCircle, Loader, RefreshCw } from 'lucide-react';

const NetworkManager = ({ nodeUrl }) => {
  const [networkInfo, setNetworkInfo] = useState(null);
  const [peers, setPeers] = useState([]);
  const [blockchainStats, setBlockchainStats] = useState(null);
  const [apiStats, setApiStats] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    fetchNetworkData();
    const interval = setInterval(fetchNetworkData, 5000);
    return () => clearInterval(interval);
  }, [nodeUrl]);

  const fetchNetworkData = async () => {
    setLoading(true);
    try {
      const [infoRes, peersRes, statsRes, apiRes] = await Promise.all([
        fetch(`${nodeUrl}/api/network/info`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/api/network/peers`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/api/blockchain/stats`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/stats`, { credentials: 'include' }).catch(() => ({ ok: false }))
      ]);

      if (infoRes.ok) {
        const data = await infoRes.json();
        setNetworkInfo(data);
      }

      if (peersRes.ok) {
        const data = await peersRes.json();
        setPeers(data.peers || []);
      }

      if (statsRes.ok) {
        const data = await statsRes.json();
        setBlockchainStats(data);
      }

      if (apiRes.ok) {
        const data = await apiRes.json();
        setApiStats(data);
      }

      setError('');
    } catch (err) {
      setError('Failed to fetch network data');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const formatNumber = (num) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return num.toString();
  };

  const formatUptime = (seconds) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  };

  const formatAddress = (addr) => {
    if (!addr) return 'N/A';
    return `${addr.slice(0, 12)}...${addr.slice(-10)}`;
  };

  return (
    <div className="space-y-6">
      {/* Header with Refresh */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold flex items-center gap-2">
          <Globe className="text-cyan-400" size={28} />
          Network Status
        </h2>
        <button
          onClick={fetchNetworkData}
          disabled={loading}
          className="p-2 bg-slate-700 hover:bg-slate-600 rounded-lg transition-all disabled:opacity-50"
          title="Refresh"
        >
          <RefreshCw size={20} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Error Display */}
      {error && (
        <div className="flex items-center gap-3 p-4 rounded-lg bg-red-900/30 border-l-4 border-red-500 text-red-200">
          <AlertCircle size={20} />
          {error}
        </div>
      )}

      {/* Network Status Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {/* Connected Peers */}
        <div className="bg-gradient-to-br from-blue-600/20 to-cyan-600/20 rounded-lg p-6 border border-blue-500/30">
          <div className="flex items-center justify-between mb-2">
            <p className="text-blue-300 text-sm">Connected Peers</p>
            <Server className="text-blue-400" size={20} />
          </div>
          <p className="text-4xl font-bold text-blue-400">{peers.length}</p>
          <p className="text-blue-400 text-xs mt-2">Active node connections</p>
        </div>

        {/* Chain Height */}
        <div className="bg-gradient-to-br from-purple-600/20 to-pink-600/20 rounded-lg p-6 border border-purple-500/30">
          <div className="flex items-center justify-between mb-2">
            <p className="text-purple-300 text-sm">Chain Height</p>
            <Zap className="text-purple-400" size={20} />
          </div>
          <p className="text-4xl font-bold text-purple-400">{blockchainStats?.height || 0}</p>
          <p className="text-purple-400 text-xs mt-2">Latest block number</p>
        </div>

        {/* Current Difficulty */}
        <div className="bg-gradient-to-br from-orange-600/20 to-red-600/20 rounded-lg p-6 border border-orange-500/30">
          <div className="flex items-center justify-between mb-2">
            <p className="text-orange-300 text-sm">Difficulty</p>
            <TrendingUp className="text-orange-400" size={20} />
          </div>
          <p className="text-4xl font-bold text-orange-400">{blockchainStats?.difficulty || 0}</p>
          <p className="text-orange-400 text-xs mt-2">Network difficulty</p>
        </div>

        {/* Mempool Size */}
        <div className="bg-gradient-to-br from-green-600/20 to-emerald-600/20 rounded-lg p-6 border border-green-500/30">
          <div className="flex items-center justify-between mb-2">
            <p className="text-green-300 text-sm">Mempool Transactions</p>
            <Activity className="text-green-400" size={20} />
          </div>
          <p className="text-4xl font-bold text-green-400">{blockchainStats?.mempool_size || 0}</p>
          <p className="text-green-400 text-xs mt-2">Pending transactions</p>
        </div>
      </div>

      {/* API Statistics */}
      {apiStats && (
        <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20">
          <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
            <Activity className="text-purple-400" size={24} />
            API Statistics
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Total Requests</p>
              <p className="text-2xl font-bold">{formatNumber(apiStats.total_requests)}</p>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Success Rate</p>
              <p className="text-2xl font-bold text-green-400">
                {apiStats.total_requests > 0 
                  ? ((apiStats.successful_requests / apiStats.total_requests) * 100).toFixed(1)
                  : '0'}%
              </p>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Uptime</p>
              <p className="text-2xl font-bold">{formatUptime(apiStats.uptime_seconds)}</p>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Blocks Mined</p>
              <p className="text-2xl font-bold text-green-400">{apiStats.blocks_mined}</p>
            </div>
          </div>

          {/* Detailed Stats */}
          <div className="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-green-900/20 rounded p-4 border border-green-500/30">
              <p className="text-green-300 text-sm mb-1">✓ Successful</p>
              <p className="text-2xl font-bold text-green-400">{formatNumber(apiStats.successful_requests)}</p>
            </div>
            <div className="bg-red-900/20 rounded p-4 border border-red-500/30">
              <p className="text-red-300 text-sm mb-1">✗ Failed</p>
              <p className="text-2xl font-bold text-red-400">{formatNumber(apiStats.failed_requests)}</p>
            </div>
            <div className="bg-yellow-900/20 rounded p-4 border border-yellow-500/30">
              <p className="text-yellow-300 text-sm mb-1">⚙ Transactions</p>
              <p className="text-2xl font-bold text-yellow-400">{formatNumber(apiStats.transactions_submitted)}</p>
            </div>
          </div>
        </div>
      )}

      {/* Network Peers */}
      <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20">
        <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
          <Network className="text-cyan-400" size={24} />
          Connected Peers ({peers.length})
        </h3>

        {peers.length === 0 ? (
          <div className="text-center p-8">
            <Server size={48} className="mx-auto text-slate-400 mb-4 opacity-50" />
            <p className="text-purple-300">No peers connected</p>
            <p className="text-purple-400 text-sm mt-2">Waiting for peer connections...</p>
          </div>
        ) : (
          <div className="space-y-3">
            {peers.map((peer, idx) => (
              <div key={idx} className="bg-slate-800/50 rounded p-4 border border-purple-500/10 hover:border-purple-500/30 transition-all">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="bg-cyan-600/20 rounded-lg p-2">
                      <Server className="text-cyan-400" size={20} />
                    </div>
                    <div>
                      <p className="text-white font-semibold font-mono text-sm">{peer.address || `Peer #${idx + 1}`}</p>
                      <p className="text-purple-400 text-xs">Connected</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="flex items-center gap-2">
                      <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                      <span className="text-green-400 text-xs font-semibold">ACTIVE</span>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Node Information */}
      <div className="bg-slate-900/50 rounded-lg p-6 border border-purple-500/20">
        <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
          <Globe className="text-cyan-400" size={24} />
          Node Information
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Network Info */}
          <div className="space-y-3">
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Node URL</p>
              <p className="font-mono text-sm text-white break-all">{nodeUrl}</p>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Network ID</p>
              <p className="font-mono text-sm text-white">TrinityChain</p>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Protocol Version</p>
              <p className="font-mono text-sm text-white">{networkInfo?.protocol_version || '1.0.0'}</p>
            </div>
          </div>

          {/* Status Info */}
          <div className="space-y-3">
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Sync Status</p>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                <p className="font-semibold text-white">Synchronized</p>
              </div>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Connection Status</p>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                <p className="font-semibold text-white">Connected</p>
              </div>
            </div>
            <div className="bg-slate-800/50 rounded p-4 border border-purple-500/10">
              <p className="text-purple-300 text-xs mb-1">Last Update</p>
              <p className="font-mono text-sm text-white">{new Date().toLocaleTimeString()}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Network Health Alert */}
      {peers.length === 0 && (
        <div className="bg-yellow-900/30 border border-yellow-500/30 rounded p-4">
          <div className="flex gap-3">
            <AlertCircle className="text-yellow-400 flex-shrink-0" size={20} />
            <div className="text-sm text-yellow-200">
              <p className="font-semibold mb-1">No Peers Connected</p>
              <p>The node is running but not connected to other peers. This is normal for a single-node setup. For network functionality, ensure other nodes are running on the network.</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default NetworkManager;
