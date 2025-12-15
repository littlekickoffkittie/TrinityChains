# Dashboard Integration Guide

## Quick Start

The enhanced TrinityChain dashboard now includes all CLI functionality through an intuitive web interface.

### Running the Dashboard

1. **Start the TrinityChain Node**:
```bash
cargo run --bin trinity-server
# or
cargo run --release --bin trinity-server
```

2. **Start the Dashboard**:
```bash
cd dashboard
npm install  # First time only
npm run dev
```

3. **Access the Dashboard**:
- Local: `http://localhost:5173`
- Remote: `http://your-server:3000` (if deployed)

## Feature Mapping: CLI → Dashboard

### Wallet Management

**CLI Commands**:
```bash
trinity-wallet new MyWallet          # Create wallet
trinity-wallet address MyWallet      # Show address
trinity-wallet list                  # List wallets
```

**Dashboard**:
- **Wallet Tab** → Create, import, export, manage wallets
- View addresses and balances
- List all wallets with balances

---

### Sending Transactions

**CLI Command**:
```bash
trinity-send <address> <amount> --from <wallet> "memo"
```

**Dashboard**:
- **Transactions Tab** → Send Transaction section
- Select wallet, enter recipient, amount, fee
- Optional memo
- Transaction summary preview

---

### Mining

**CLI Command**:
```bash
trinity-miner <miner_address>
```

**Dashboard**:
- **Mining Tab** → Start Mining section
- Select wallet or enter custom address
- One-click start/stop
- Real-time stats: blocks mined, difficulty, rewards

---

### Checking Balance

**CLI Command**:
```bash
trinity-balance <address>
```

**Dashboard**:
- **Wallet Tab** → Select wallet → View balance
- **Transactions Tab** → Automatic balance display
- Real-time updates

---

### Viewing Transaction History

**CLI Command**:
```bash
trinity-history <address>
```

**Dashboard**:
- **Transactions Tab** → History section
- All transactions for selected wallet
- Timestamp and details for each

---

### Block Explorer

**CLI Equivalent**: 
```bash
# No direct CLI equivalent
```

**Dashboard**:
- **Block Explorer Tab** → Search and view blocks
- Search by height, hash, or previous hash
- Expandable block details
- Transaction listing per block

---

### Network Information

**CLI Equivalent**:
```bash
# No direct CLI equivalent
```

**Dashboard**:
- **Network Tab** → Full network monitoring
- Connected peers
- Blockchain stats
- API statistics
- Node health

---

### Mining Status & Statistics

**CLI Equivalent**:
```bash
# Miner provides terminal UI
```

**Dashboard**:
- **Mining Tab** → Complete mining metrics
- Real-time stats updates
- Performance tracking
- Reward information

---

## API Architecture

### Client-Server Flow

```
Dashboard (Browser)
    ↓
REST API (Node.js on Rust backend)
    ↓
TrinityChain Core (Rust)
    ↓
Blockchain State
```

### Key API Endpoints

All requests to: `http://localhost:3000/api/`

| Operation | Endpoint | Method |
|-----------|----------|--------|
| Create Wallet | `/wallet/create` | POST |
| Get Balance | `/address/:addr/balance` | GET |
| Send Transaction | `/transaction` | POST |
| Start Mining | `/mining/start` | POST |
| Stop Mining | `/mining/stop` | POST |
| Get Stats | `/blockchain/stats` | GET |
| Get Blocks | `/blockchain/blocks` | GET |
| Get Peers | `/network/peers` | GET |

## Configuration

### Environment Variables

Create `.env.local` in `dashboard/` (if needed):

```env
VITE_API_URL=http://localhost:3000
VITE_AUTO_REFRESH=true
VITE_REFRESH_INTERVAL=3000
```

### Node Configuration

Ensure your TrinityChain node is running with API enabled:

```toml
# config.toml
[api]
enabled = true
port = 3000
host = "0.0.0.0"  # For remote access
```

## Deployment

### Development
```bash
npm run dev
```

### Production Build
```bash
npm run build
npm run preview
```

### Docker Deployment

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY dashboard .
RUN npm install && npm run build
EXPOSE 5173
CMD ["npm", "run", "preview"]
```

### Render Deployment

See `deployment/render.yaml` for pre-configured deployment settings.

## Troubleshooting

### Dashboard won't connect to node

1. Check node is running: `ps aux | grep trinity`
2. Verify API is enabled in config
3. Check URL in dashboard settings
4. Verify firewall allows port 3000

### Wallets not saving

1. Enable localStorage in browser
2. Check browser storage quota
3. Check console for errors (F12)
4. Try exporting wallet as backup

### Mining not working

1. Verify miner address is valid
2. Check node has genesis block
3. Monitor node logs for errors
4. Ensure sufficient peers (optional for single-node)

### Transactions failing

1. Check sender address exists
2. Verify sufficient balance
3. Check recipient address format
4. Monitor mempool for congestion

## Performance Optimization

### Dashboard Performance

1. Adjust refresh interval:
   - Lower for real-time data: 1000ms
   - Higher for bandwidth: 10000ms

2. Monitor API calls:
   - Open DevTools Network tab
   - Check for failed requests
   - Verify response times

3. Browser optimization:
   - Use Chrome/Edge for best performance
   - Clear cache if experiencing issues
   - Disable unnecessary extensions

### Node Performance

1. Increase worker threads for mining
2. Optimize blockchain storage
3. Configure appropriate difficulty
4. Monitor memory usage

## Security Best Practices

⚠️ **Critical**:

1. **Never expose private keys** in the UI
2. **Use HTTPS** in production
3. **Validate all inputs** on server-side
4. **Rate limit API** endpoints
5. **Backup wallets** regularly
6. **Use secure connection** to node
7. **Enable authentication** for production

## Integration Examples

### JavaScript/Fetch

```javascript
// Fetch balance
const response = await fetch('http://localhost:3000/api/address/0x.../balance');
const data = await response.json();
console.log(data.balance);

// Start mining
const response = await fetch('http://localhost:3000/api/mining/start', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ miner_address: '0x...' })
});
```

### Python Integration

```python
import requests

# Get blockchain stats
response = requests.get('http://localhost:3000/api/blockchain/stats')
stats = response.json()
print(f"Chain height: {stats['height']}")

# Send transaction
tx = {
    "Transfer": {
        "input_hash": "0x...",
        "new_owner": "0x...",
        "sender": "0x...",
        "amount": "100.0",
        "fee_area": "1.0",
        "nonce": 0,
        "signature": "0x...",
        "public_key": "0x..."
    }
}
response = requests.post('http://localhost:3000/api/transaction', json=tx)
```

## Support & Documentation

- **API Docs**: See `documentation/API_ENDPOINTS.md`
- **Dashboard Docs**: See `documentation/DASHBOARD_FEATURES.md`
- **Architecture**: See `documentation/ARCHITECTURE_AUDIT.md`
- **Issues**: Check GitHub issues or create new ones

## Version History

- **v1.0.0** (December 2025) - Initial release with full CLI functionality
  - Wallet management
  - Transaction sending
  - Mining controls
  - Network monitoring
  - Block explorer
  - Analytics dashboard

---

**Status**: ✅ Production Ready
**Last Updated**: December 2025
