import React, { useEffect, useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';

const DebugComponent = () => {
  const [debugInfo, setDebugInfo] = useState({
    nodeUrl: '',
    apiStatus: 'checking...',
    statsData: null,
    error: null,
    browserInfo: {},
    endpoints: {},
    corsHeaders: {},
    responseTimes: {},
    localStorage: {},
    performance: {},
    detailedLogs: [],
  });
  
  const [expandedSections, setExpandedSections] = useState({
    overview: true,
    endpoints: true,
    cors: false,
    browser: false,
    storage: false,
    performance: false,
    logs: false,
  });

  const toggleSection = (section) => {
    setExpandedSections(prev => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const addLog = (message, type = 'info') => {
    setDebugInfo(prev => ({
      ...prev,
      detailedLogs: [...prev.detailedLogs, {
        timestamp: new Date().toLocaleTimeString(),
        message,
        type,
      }].slice(-20), // Keep last 20 logs
    }));
  };

  useEffect(() => {
    const test = async () => {
      // Get NodeUrl
      const getNodeUrl = () => {
        if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
          return 'http://localhost:3000';
        }
        if (window.location.hostname.includes('.github.dev')) {
          return window.location.origin.replace('-5173.', '-3000.');
        }
        if (window.location.hostname.includes('render.com') || window.location.hostname.includes('vercel.app')) {
          return window.location.origin;
        }
        return `${window.location.protocol}//${window.location.hostname}:3000`;
      };

      const nodeUrl = getNodeUrl();
      
      addLog('🔍 Debug initialization started', 'info');
      addLog(`Node URL determined: ${nodeUrl}`, 'info');
      addLog(`Window location: ${window.location.href}`, 'info');
      addLog(`Hostname: ${window.location.hostname}`, 'info');

      // Collect browser info
      const browserInfo = {
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        language: navigator.language,
        onLine: navigator.onLine,
        cookieEnabled: navigator.cookieEnabled,
        maxTouchPoints: navigator.maxTouchPoints,
      };
      addLog(`Browser: ${browserInfo.userAgent.substring(0, 50)}...`, 'info');

      // Collect local storage info
      const localStorage = {};
      try {
        for (let i = 0; i < window.localStorage.length; i++) {
          const key = window.localStorage.key(i);
          const value = window.localStorage.getItem(key);
          localStorage[key] = value?.substring(0, 100) + (value?.length > 100 ? '...' : '');
        }
      } catch (e) {
        addLog(`LocalStorage error: ${e.message}`, 'error');
      }

      // Test multiple endpoints
      const endpoints = {};
      const endpointsToTest = [
        '/api/blockchain/stats',
        '/api/blockchain/height',
        '/api/mining/status',
        '/api/network/peers',
        '/api/health',
        '/api/stats',
      ];

      const responseTimes = {};
      const corsHeaders = {};

      for (const endpoint of endpointsToTest) {
        try {
          addLog(`Testing ${endpoint}...`, 'info');
          const startTime = performance.now();

          const response = await fetch(`${nodeUrl}${endpoint}`, {
            credentials: 'include',
          });

          const endTime = performance.now();
          const responseTime = (endTime - startTime).toFixed(2);
          responseTimes[endpoint] = `${responseTime}ms`;

          // Collect CORS headers
          corsHeaders[endpoint] = {
            contentType: response.headers.get('content-type'),
            accessControlAllowOrigin: response.headers.get('access-control-allow-origin'),
            accessControlAllowCredentials: response.headers.get('access-control-allow-credentials'),
            accessControlAllowMethods: response.headers.get('access-control-allow-methods'),
            accessControlAllowHeaders: response.headers.get('access-control-allow-headers'),
          };

          if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
          }

          const data = await response.json();
          endpoints[endpoint] = {
            status: '✓ OK',
            statusCode: response.status,
            dataKeys: Object.keys(data).slice(0, 5).join(', '),
          };
          addLog(`✓ ${endpoint} succeeded in ${responseTime}ms`, 'success');
        } catch (error) {
          endpoints[endpoint] = {
            status: '✗ FAILED',
            error: error.message,
          };
          addLog(`✗ ${endpoint} failed: ${error.message}`, 'error');
        }
      }

      // Performance metrics
      const performance_metrics = {
        navigationStart: performance.timing?.navigationStart,
        pageLoadTime: performance.timing?.loadEventEnd - performance.timing?.navigationStart,
        domContentLoaded: performance.timing?.domContentLoadedEventEnd - performance.timing?.navigationStart,
        memory: performance.memory ? {
          usedJSHeapSize: `${(performance.memory.usedJSHeapSize / 1048576).toFixed(2)} MB`,
          jsHeapSizeLimit: `${(performance.memory.jsHeapSizeLimit / 1048576).toFixed(2)} MB`,
        } : 'Not available',
      };

      const overallStatus = Object.values(endpoints).some(e => e.status === '✓ OK') ? 'Connected ✓' : 'Failed ✗';

      setDebugInfo(prev => ({
        ...prev,
        nodeUrl,
        apiStatus: overallStatus,
        statsData: endpoints['/api/blockchain/stats'],
        browserInfo,
        endpoints,
        corsHeaders,
        responseTimes,
        localStorage,
        performance: performance_metrics,
      }));

      addLog('✅ Debug information collection complete', 'success');
    };

    test();
  }, []);

  const SectionHeader = ({ title, section }) => (
    <button
      onClick={() => toggleSection(section)}
      className="w-full flex items-center gap-2 py-2 px-3 hover:bg-slate-700 rounded text-left font-semibold text-blue-400"
    >
      {expandedSections[section] ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
      {title}
    </button>
  );

  const StatusBadge = ({ status }) => {
    const isSuccess = status.includes('✓');
    return (
      <span className={`px-2 py-1 rounded text-xs font-bold ${
        isSuccess ? 'bg-green-900 text-green-200' : 'bg-red-900 text-red-200'
      }`}>
        {status}
      </span>
    );
  };

  return (
    <div style={{
      padding: '16px',
      backgroundColor: '#0f172a',
      color: '#e2e8f0',
      borderRadius: '8px',
      fontFamily: 'monospace',
      fontSize: '11px',
      maxHeight: '600px',
      overflow: 'auto',
      border: '1px solid #334155',
      lineHeight: '1.4',
    }}>
      <h2 style={{ margin: '0 0 12px 0', color: '#60a5fa', fontSize: '14px' }}>
        🔍 Dashboard Debug Info
      </h2>

      {/* OVERVIEW SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="📊 Overview" section="overview" />
        {expandedSections.overview && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', marginTop: '8px' }}>
              <div>
                <div style={{ color: '#94a3b8' }}>Node URL:</div>
                <div style={{ color: '#fbbf24', wordBreak: 'break-all' }}>{debugInfo.nodeUrl}</div>
              </div>
              <div>
                <div style={{ color: '#94a3b8' }}>API Status:</div>
                <StatusBadge status={debugInfo.apiStatus} />
              </div>
            </div>
            {debugInfo.error && (
              <div style={{ marginTop: '8px', color: '#ff4444' }}>
                <strong>Error:</strong> {debugInfo.error}
              </div>
            )}
          </div>
        )}
      </div>

      {/* ENDPOINTS SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="🔗 Endpoints Test Results" section="endpoints" />
        {expandedSections.endpoints && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            {Object.entries(debugInfo.endpoints).map(([endpoint, result]) => (
              <div key={endpoint} style={{ marginTop: '6px', padding: '6px', backgroundColor: '#1e293b', borderRadius: '4px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                  <span style={{ color: '#60a5fa' }}>{endpoint}</span>
                  <StatusBadge status={result.status} />
                </div>
                {result.statusCode && <div style={{ color: '#94a3b8' }}>Status Code: {result.statusCode}</div>}
                {result.dataKeys && <div style={{ color: '#94a3b8' }}>Data: {result.dataKeys}...</div>}
                {result.error && <div style={{ color: '#ff6b6b' }}>Error: {result.error}</div>}
                {debugInfo.responseTimes[endpoint] && (
                  <div style={{ color: '#86efac' }}>Response Time: {debugInfo.responseTimes[endpoint]}</div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* CORS HEADERS SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="🔐 CORS Headers" section="cors" />
        {expandedSections.cors && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            {Object.entries(debugInfo.corsHeaders).slice(0, 2).map(([endpoint, headers]) => (
              <div key={endpoint} style={{ marginTop: '8px', padding: '6px', backgroundColor: '#1e293b', borderRadius: '4px' }}>
                <div style={{ color: '#60a5fa', fontWeight: 'bold', marginBottom: '4px' }}>{endpoint}</div>
                {Object.entries(headers).map(([key, value]) => (
                  <div key={key} style={{ fontSize: '10px', color: value ? '#86efac' : '#ff6b6b' }}>
                    {key}: {value || '(not set)'}
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* BROWSER INFO SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="🌐 Browser Info" section="browser" />
        {expandedSections.browser && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            {Object.entries(debugInfo.browserInfo).map(([key, value]) => (
              <div key={key} style={{ fontSize: '10px', marginTop: '4px' }}>
                <span style={{ color: '#94a3b8' }}>{key}:</span> <span style={{ color: '#fbbf24' }}>{String(value)}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* STORAGE SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="💾 LocalStorage" section="storage" />
        {expandedSections.storage && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            {Object.keys(debugInfo.localStorage).length === 0 ? (
              <div style={{ color: '#94a3b8', fontSize: '10px' }}>No data in localStorage</div>
            ) : (
              Object.entries(debugInfo.localStorage).map(([key, value]) => (
                <div key={key} style={{ fontSize: '10px', marginTop: '4px', wordBreak: 'break-all' }}>
                  <span style={{ color: '#60a5fa' }}>{key}:</span> <span style={{ color: '#86efac' }}>{value}</span>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* PERFORMANCE SECTION */}
      <div style={{ marginBottom: '8px', borderBottom: '1px solid #334155' }}>
        <SectionHeader title="⚡ Performance" section="performance" />
        {expandedSections.performance && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px' }}>
            {Object.entries(debugInfo.performance).map(([key, value]) => (
              <div key={key} style={{ fontSize: '10px', marginTop: '4px' }}>
                <span style={{ color: '#94a3b8' }}>{key}:</span>
                {typeof value === 'object' ? (
                  <div style={{ marginLeft: '8px' }}>
                    {Object.entries(value).map(([k, v]) => (
                      <div key={k} style={{ fontSize: '9px', color: '#fbbf24' }}>{k}: {v}</div>
                    ))}
                  </div>
                ) : (
                  <span style={{ color: '#fbbf24' }}>{value} ms</span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* DETAILED LOGS SECTION */}
      <div style={{ marginBottom: '8px' }}>
        <SectionHeader title="📝 Detailed Logs" section="logs" />
        {expandedSections.logs && (
          <div style={{ paddingLeft: '16px', paddingBottom: '8px', maxHeight: '150px', overflow: 'auto' }}>
            {debugInfo.detailedLogs.length === 0 ? (
              <div style={{ color: '#94a3b8', fontSize: '10px' }}>No logs yet</div>
            ) : (
              debugInfo.detailedLogs.map((log, idx) => (
                <div key={idx} style={{
                  fontSize: '9px',
                  marginTop: '2px',
                  color: log.type === 'success' ? '#86efac' : log.type === 'error' ? '#ff6b6b' : '#94a3b8',
                  display: 'flex',
                  gap: '8px',
                }}>
                  <span style={{ color: '#475569', minWidth: '60px' }}>{log.timestamp}</span>
                  <span>{log.message}</span>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      <div style={{ marginTop: '8px', fontSize: '9px', color: '#475569', textAlign: 'center' }}>
        Press F12 to open DevTools for more details
      </div>
    </div>
  );
};

export default DebugComponent;
