// Telegram Web App initialization
let tg = window.Telegram?.WebApp;

// API base URL - can be configured via Telegram Bot `start_param`, query param `?api=` or defaults to relative
function resolveApiBase() {
    // 1) start_param (bot can set start_param to the API base)
    if (tg && tg.initDataUnsafe && tg.initDataUnsafe.start_param) {
        return tg.initDataUnsafe.start_param;
    }

    // 2) query param ?api=
    try {
        const urlParams = new URLSearchParams(window.location.search);
        const apiParam = urlParams.get('api');
        if (apiParam) return apiParam;
    } catch (e) {
        // ignore
    }

    // 3) fallback to /api (relative path for same-server deployments)
    return '/api';
}

const API_BASE = resolveApiBase();

// Debug logging
console.log('🔺 TrinityChain Dashboard Debug Info:');
console.log('API_BASE:', API_BASE);
console.log('URL:', window.location.href);
console.log('Telegram Web App:', tg ? 'Available' : 'Not available');
if (tg && tg.initDataUnsafe) {
    console.log('User:', tg.initDataUnsafe.user?.username || tg.initDataUnsafe.user?.first_name || 'Unknown');
}

// Initialize Telegram Web App
if (tg) {
    tg.ready();
    tg.expand();

    // Apply Telegram theme colors
    document.documentElement.style.setProperty('--tg-theme-bg-color', tg.themeParams.bg_color || '#0a0e27');
    document.documentElement.style.setProperty('--tg-theme-text-color', tg.themeParams.text_color || '#e0e0e0');
    document.documentElement.style.setProperty('--tg-theme-hint-color', tg.themeParams.hint_color || '#888');
    document.documentElement.style.setProperty('--tg-theme-link-color', tg.themeParams.link_color || '#00ff88');
    document.documentElement.style.setProperty('--tg-theme-button-color', tg.themeParams.button_color || '#00ff88');
    document.documentElement.style.setProperty('--tg-theme-button-text-color', tg.themeParams.button_text_color || '#ffffff');
    document.documentElement.style.setProperty('--tg-theme-secondary-bg-color', tg.themeParams.secondary_bg_color || '#1a1f3a');
}

// Format timestamp
function formatTime(timestamp) {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
}

// Format hash (show first 8 and last 8 characters)
function formatHash(hash) {
    if (hash.length > 16) {
        return `${hash.substring(0, 8)}...${hash.substring(hash.length - 8)}`;
    }
    return hash;
}

// Fetch and update dashboard data
async function updateDashboardData() {
    const statsUrl = `${API_BASE}/blockchain/stats`;
    console.log(`[updateDashboardData] Fetching from: ${statsUrl}`);
    try {
        const response = await fetch(statsUrl);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const statsData = await response.json();

        // Update stats
        document.getElementById('blockHeight').textContent = statsData.height || '0';
        document.getElementById('totalTriangles').textContent = statsData.utxo_count || '0';
        document.getElementById('difficulty').textContent = statsData.difficulty || '2';
        document.getElementById('totalArea').textContent = (statsData.utxo_count * 100).toFixed(2);

        // Update recent blocks
        const blocksContainer = document.getElementById('recentBlocks');
        if (!statsData.recent_blocks || statsData.recent_blocks.length === 0) {
            blocksContainer.innerHTML = '<p class="loading">No blocks yet.</p>';
        } else {
            const blockPromises = statsData.recent_blocks.slice(0, 5).map(async (blockInfo) => {
                try {
                    const blockResponse = await fetch(`${API_BASE}/blockchain/block/by-height/${blockInfo.height}`);
                    return await blockResponse.json();
                } catch (e) {
                    return null;
                }
            });
            const blocks = (await Promise.all(blockPromises)).filter(b => b !== null);
            blocksContainer.innerHTML = blocks.map((block, index) => {
                const blockInfo = statsData.recent_blocks[index];
                return `
                    <div class="block-item" onclick="fetchBlockDetails(${block.header.height})">
                        <div class="block-header">
                            <span class="block-height">Block #${block.header.height}</span>
                            <span class="block-time">${formatTime(block.header.timestamp)}</span>
                        </div>
                        <div class="block-hash">Hash: ${formatHash(blockInfo.hash)}</div>
                        <div style="margin-top: 10px; color: #888; font-size: 0.9rem;">
                            ${block.transactions.length} tx(s) • Difficulty: ${block.header.difficulty}
                        </div>
                    </div>`;
            }).join('');
        }
        if (tg) tg.HapticFeedback.impactOccurred('light');
    } catch (error) {
        console.error('[updateDashboardData] Error:', error);
        ['blockHeight', 'totalTriangles', 'totalArea', 'difficulty'].forEach(id => {
            document.getElementById(id).textContent = 'Offline';
        });
        document.getElementById('recentBlocks').innerHTML = '<p class="loading">API server offline.</p>';
        if (tg) tg.showAlert('Unable to connect to the blockchain API.');
    }
}

// --- Mining Control Logic ---

const minerAddressInput = document.getElementById('minerAddressInput');
const loadCliWalletButton = document.getElementById('loadCliWalletButton');
const startMiningButton = document.getElementById('startMiningButton');
const stopMiningButton = document.getElementById('stopMiningButton');
const minerAddressDisplay = document.getElementById('minerAddress');
const miningStatusDisplay = document.getElementById('miningStatus');
const blocksMinedDisplay = document.getElementById('blocksMined');
const hashRateDisplay = document.getElementById('hashRate');

// Inject CLI wallet address from Python kernel to make it available in JS
let cliMinerAddress = '7339ba1f28a194fe5d099a9d7551e1aa78a633e85f3c846b4e046d7cbe43f434';

function updateMiningStatusDisplay(status) {
    miningStatusDisplay.innerText = status.is_mining ? 'Active' : 'Inactive';
    blocksMinedDisplay.innerText = status.blocks_mined;
    hashRateDisplay.innerText = `${status.hashrate.toFixed(2)} H/s`;
    miningStatusDisplay.style.color = status.is_mining ? '#00ff88' : '#ff0044';

    // Update button states based on mining status
    startMiningButton.disabled = status.is_mining;
    stopMiningButton.disabled = !status.is_mining;
    loadCliWalletButton.disabled = status.is_mining;
    minerAddressInput.disabled = status.is_mining;
}

async function fetchMiningStatus() {
    try {
        const response = await fetch(`${API_BASE}/mining/status`);
        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`HTTP error! status: ${response.status}, message: ${errorText}`);
        }
        const status = await response.json();
        updateMiningStatusDisplay(status);
    } catch (error) {
        console.error('Error fetching mining status:', error);
        miningStatusDisplay.innerText = 'Error';
        miningStatusDisplay.style.color = '#ff0044';
        startMiningButton.disabled = false;
        stopMiningButton.disabled = true;
        loadCliWalletButton.disabled = false;
        minerAddressInput.disabled = false;
    }
}

async function startMining() {
    const address = minerAddressInput.value.trim();
    if (!address) {
        alert('Please enter a miner address.');
        return;
    }
    try {
        startMiningButton.disabled = true;
        stopMiningButton.disabled = true; // Disable until status is confirmed
        loadCliWalletButton.disabled = true;
        minerAddressInput.disabled = true;

        const response = await fetch(`${API_BASE}/mining/start`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ miner_address: address })
        });
        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`HTTP error! status: ${response.status}, message: ${errorText}`);
        }
        await response.text();
        fetchMiningStatus();
    } catch (error) {
        console.error('Error starting mining:', error);
        alert(`Failed to start mining: ${error.message}`);
        fetchMiningStatus();
    }
}

async function stopMining() {
    try {
        stopMiningButton.disabled = true;
        startMiningButton.disabled = true; // Disable until status is confirmed
        loadCliWalletButton.disabled = true;
        minerAddressInput.disabled = true;

        const response = await fetch(`${API_BASE}/mining/stop`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });
        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`HTTP error! status: ${response.status}, message: ${errorText}`);
        }
        await response.text();
        fetchMiningStatus();
    } catch (error) {
        console.error('Error stopping mining:', error);
        alert(`Failed to stop mining: ${error.message}`);
        fetchMiningStatus();
    }
}

function loadCliWalletAddress() {
    if (cliMinerAddress) {
        minerAddressInput.value = cliMinerAddress;
        minerAddressDisplay.innerText = formatHash(cliMinerAddress);
        if(tg) tg.showAlert('CLI wallet address loaded!');
    } else {
        if(tg) tg.showAlert('CLI wallet address not available.');
    }
}

function setupMiningControls() {
    if (minerAddressInput && loadCliWalletButton && startMiningButton && stopMiningButton) {
        loadCliWalletButton.addEventListener('click', loadCliWalletAddress);
        startMiningButton.addEventListener('click', startMining);
        stopMiningButton.addEventListener('click', stopMining);

        if (cliMinerAddress) {
            minerAddressInput.value = cliMinerAddress;
            minerAddressDisplay.innerText = formatHash(cliMinerAddress);
        } else {
            minerAddressDisplay.innerText = 'Not Set';
        }

        fetchMiningStatus();
        setInterval(fetchMiningStatus, 5000);
    }
}
// --- End Mining Control Logic ---

// Fetch and display block details in the modal
async function fetchBlockDetails(blockHeight) {
    const modal = document.getElementById('blockDetailsModal');
    const content = document.getElementById('blockDetailsContent');
    content.innerHTML = '<p class="loading">Loading block details...</p>';
    modal.style.display = 'block';

    try {
        const response = await fetch(`${API_BASE}/blockchain/block/by-height/${blockHeight}`);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const block = await response.json();

        let transactionsHtml = '<h4>Transactions</h4>';
        if (block.transactions.length === 0) {
            transactionsHtml += '<p>No transactions in this block.</p>';
        } else {
            transactionsHtml += block.transactions.map(tx => `
                <div class="transaction-item">
                    <p><strong>Hash:</strong> ${formatHash(tx.hash)}</p>
                    <p><strong>Timestamp:</strong> ${formatTime(tx.timestamp)}</p>
                </div>
            `).join('');
        }

        content.innerHTML = `
            <h3>Block #${block.header.height}</h3>
            <p><strong>Hash:</strong> ${formatHash(block.header.hash)}</p>
            <p><strong>Previous Hash:</strong> ${formatHash(block.header.previous_hash)}</p>
            <p><strong>Timestamp:</strong> ${formatTime(block.header.timestamp)}</p>
            <p><strong>Difficulty:</strong> ${block.header.difficulty}</p>
            <p><strong>Nonce:</strong> ${block.header.nonce}</p>
            <hr>
            ${transactionsHtml}
        `;
    } catch (error) {
        console.error('[fetchBlockDetails] Error:', error);
        content.innerHTML = '<p class="loading">Unable to fetch block details.</p>';
    }
}

// Update data periodically
function startAutoUpdate() {
    updateDashboardData();
    setInterval(updateDashboardData, 10000);
}

// Start when page loads
document.addEventListener('DOMContentLoaded', () => {
    // Update debug panel
    document.getElementById('debugApiBase').textContent = API_BASE;
    document.getElementById('debugStatus').textContent = 'Fetching data...';
    
    startAutoUpdate();
    setupMiningControls();

    // Log Telegram user info if available
    if (tg && tg.initDataUnsafe.user) {
        console.log('Telegram User:', tg.initDataUnsafe.user.username || tg.initDataUnsafe.user.first_name);
    }

    // Modal close logic
    const modal = document.getElementById('blockDetailsModal');
    const closeButton = document.querySelector('.close-button');

    closeButton.onclick = function() {
        modal.style.display = 'none';
    }

    window.onclick = function(event) {
        if (event.target == modal) {
            modal.style.display = 'none';
        }
    }

    // Search logic
    const searchButton = document.getElementById('searchButton');
    searchButton.addEventListener('click', search);

    // Wallet controls
    const createWalletButton = document.getElementById('createWalletButton');
    const loadWalletButton = document.getElementById('loadWalletButton');
    const sendButton = document.getElementById('sendButton');
    const loadWalletModal = document.getElementById('loadWalletModal');
    const loadWalletFromJsonButton = document.getElementById('loadWalletFromJsonButton');
    const walletJsonInput = document.getElementById('walletJsonInput');

    createWalletButton.addEventListener('click', createWallet);
    loadWalletButton.addEventListener('click', () => loadWalletModal.style.display = 'block');
    loadWalletFromJsonButton.addEventListener('click', () => loadWallet(walletJsonInput.value));
    sendButton.addEventListener('click', sendTransaction);

    // Modal close logic
    const modals = document.getElementsByClassName('modal');
    for (let i = 0; i < modals.length; i++) {
        const modal = modals[i];
        const closeButton = modal.querySelector('.close-button');
        closeButton.onclick = function() {
            modal.style.display = 'none';
        }
        window.onclick = function(event) {
            if (event.target == modal) {
                modal.style.display = 'none';
            }
        }
    }

    initPriceChart();
});

// Toggle debug panel visibility
function toggleDebugPanel() {
    const panel = document.getElementById('debugPanel');
    panel.style.display = panel.style.display === 'none' ? 'block' : 'none';
}

// Search for a block by height or hash
async function search() {
    const searchInput = document.getElementById('searchInput');
    const query = searchInput.value.trim();
    if (!query) {
        return;
    }

    const searchResults = document.getElementById('searchResults');
    searchResults.innerHTML = '<p class="loading">Searching...</p>';

    try {
        let response;
        if (isNaN(query)) {
            // Search by hash
            response = await fetch(`${API_BASE}/blockchain/block/by-hash/${query}`);
        } else {
            // Search by height
            response = await fetch(`${API_BASE}/blockchain/block/by-height/${query}`);
        }

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        const block = await response.json();
        searchResults.innerHTML = `
            <div class="block-item" onclick="fetchBlockDetails(${block.header.height})">
                <div class="block-header">
                    <span class="block-height">Block #${block.header.height}</span>
                    <span class="block-time">${formatTime(block.header.timestamp)}</span>
                </div>
                <div class="block-hash">
                    Hash: ${formatHash(block.header.hash)}
                </div>
                <div style="margin-top: 10px; color: #888; font-size: 0.9rem;">
                    ${block.transactions.length} transaction(s) • Difficulty: ${block.header.difficulty}
                </div>
            </div>
        `;
    } catch (error) {
        console.error('[search] Error:', error);
        searchResults.innerHTML = '<p class="loading">Block not found.</p>';
    }
}

// Fetch price data from CoinGecko
async function fetchPriceData() {
    try {
        const response = await fetch('https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd');
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const data = await response.json();
        return data.bitcoin.usd;
    } catch (error) {
        console.error('[fetchPriceData] Error:', error);
        return null;
    }
}

// Initialize and update the price chart
async function initPriceChart() {
    const ctx = document.getElementById('priceChart').getContext('2d');
    const priceChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [{
                label: 'Price (USD)',
                data: [],
                borderColor: 'rgba(0, 255, 136, 1)',
                backgroundColor: 'rgba(0, 255, 136, 0.2)',
                borderWidth: 1
            }]
        },
        options: {
            scales: {
                x: {
                    type: 'time',
                    time: {
                        unit: 'second'
                    }
                },
                y: {
                    beginAtZero: true
                }
            }
        }
    });

    // Fetch and update price data every 60 seconds
    setInterval(async () => {
        const price = await fetchPriceData();
        if (price) {
            const now = new Date();
            priceChart.data.labels.push(now);
            priceChart.data.datasets[0].data.push(price);
            priceChart.update();
        }
    }, 60000);
}

// Tab functionality
function openTab(evt, tabName) {
    var i, tabcontent, tablinks;
    tabcontent = document.getElementsByClassName("tab-content");
    for (i = 0; i < tabcontent.length; i++) {
        tabcontent[i].style.display = "none";
    }
    tablinks = document.getElementsByClassName("tab-link");
    for (i = 0; i < tablinks.length; i++) {
        tablinks[i].className = tablinks[i].className.replace(" active", "");
    }
    document.getElementById(tabName).style.display = "block";
    evt.currentTarget.className += " active";
}
