# Dashboard Testing Guide

## Pre-requisites

1. **TrinityChain Node Running**
   ```bash
   cargo run --release --bin trinity-server
   ```

2. **Dashboard Dependencies Installed**
   ```bash
   cd dashboard
   npm install
   ```

## Testing Workflow

### 1. Start the Dashboard

```bash
cd dashboard
npm run dev
```

Dashboard will be available at: `http://localhost:5173`

### 2. Verify Node Connection

1. Open dashboard in browser
2. Look for green "Live" indicator in top-right
3. Check **Settings** → Node URL is correct
4. Should see blockchain stats populated

**Expected Result**: ✅ Green status indicator, stats displaying

---

## Feature Testing Checklist

### Wallet Management Tests ✅

#### Test 1: Create Wallet
1. Click **Wallet** tab
2. Click **Create Wallet** button
3. Enter name "test-wallet-1"
4. Click **Create**

**Expected Result**: 
- ✅ Success message appears
- ✅ Wallet listed below
- ✅ Address displayed
- ✅ Balance showing

#### Test 2: Export Wallet
1. Hover over created wallet
2. Click **Download** (export) button
3. Verify JSON file downloaded

**Expected Result**:
- ✅ File downloads as `test-wallet-1-wallet.json`
- ✅ Contains address and public key

#### Test 3: Import Wallet
1. Click **Import Wallet**
2. Name it "imported-wallet"
3. Paste exported JSON content
4. Click **Import**

**Expected Result**:
- ✅ Import succeeds
- ✅ Wallet appears in list
- ✅ Same address as exported wallet

#### Test 4: View Details
1. Click wallet in list
2. Verify address displays
3. Click eye icon on public key
4. Verify key visibility toggles

**Expected Result**:
- ✅ Wallet expands
- ✅ Public key visible/hidden on toggle
- ✅ Copy buttons work

#### Test 5: Delete Wallet
1. Click delete icon (trash) on wallet
2. Confirm deletion
3. Verify wallet removed from list

**Expected Result**:
- ✅ Confirmation dialog appears
- ✅ Wallet deleted after confirmation
- ✅ Removed from list immediately

---

### Transaction Tests ✅

#### Test 1: View Balance
1. Click **Transactions** tab
2. Select a wallet from dropdown
3. Verify balance displays

**Expected Result**:
- ✅ Balance shows (even if 0)
- ✅ Updates from blockchain

#### Test 2: Send Transaction (if balance available)
1. Go to **Transactions** → **Send** tab
2. Select wallet with balance
3. Enter recipient address (another wallet)
4. Enter amount
5. Set fee (e.g., 1.0)
6. Click **Send Transaction**

**Expected Result**:
- ✅ Success/error message appears
- ✅ Balance updates
- ✅ Transaction appears in history tab

#### Test 3: Transaction History
1. Click **History** tab
2. Verify transactions display for selected wallet

**Expected Result**:
- ✅ Transactions listed
- ✅ Shows hash, amount, timestamp

#### Test 4: Mempool Monitoring
1. Click **Mempool** tab
2. Should see pending transactions

**Expected Result**:
- ✅ Pending transactions listed
- ✅ Count shown in header
- ✅ Auto-refreshes every 5 seconds

---

### Mining Tests ✅

#### Test 1: Configure Mining
1. Click **Mining** tab
2. Select wallet from dropdown OR
3. Enter custom miner address
4. Verify address displays

**Expected Result**:
- ✅ Configuration ready
- ✅ Address field shows selection

#### Test 2: Start Mining
1. Click **Start Mining** button
2. Observe status change to "Mining Active"
3. Check button changes to "Stop Mining"
4. Wait for mining to complete a block

**Expected Result**:
- ✅ Button changes color (green)
- ✅ Status indicator shows "Mining Active"
- ✅ "Blocks Mined" counter increases
- ✅ Chain height increases

#### Test 3: Monitor Mining Stats
1. While mining, watch stats update
2. Observe real-time metrics:
   - Blocks Mined
   - Chain Height
   - Difficulty
   - Mempool Size

**Expected Result**:
- ✅ Stats update every 2 seconds
- ✅ Block mined counter increments
- ✅ Performance metrics display

#### Test 4: Stop Mining
1. Click **Stop Mining** button
2. Verify button changes back

**Expected Result**:
- ✅ Mining stops immediately
- ✅ Button returns to "Start Mining"
- ✅ Status shows "Mining Inactive"
- ✅ Stats stop updating

#### Test 5: Reward Distribution
1. Mine a few blocks
2. Check recipient wallet balance
3. Should increase by block rewards

**Expected Result**:
- ✅ Miner address receives rewards
- ✅ Balance increases in Wallet tab

---

### Network Monitoring Tests ✅

#### Test 1: View Network Status
1. Click **Network** tab
2. Verify cards showing:
   - Connected Peers (should be 0+ for single node)
   - Chain Height
   - Difficulty
   - Mempool Transactions

**Expected Result**:
- ✅ All stats display
- ✅ Numbers match blockchain stats

#### Test 2: API Statistics
1. Scroll to "API Statistics" section
2. Verify displaying:
   - Total Requests
   - Success Rate
   - Uptime
   - Blocks Mined

**Expected Result**:
- ✅ API stats panel shows metrics
- ✅ Success rate calculated correctly

#### Test 3: Node Information
1. Scroll to "Node Information" section
2. Verify displaying:
   - Node URL
   - Protocol Version
   - Sync Status
   - Connection Status

**Expected Result**:
- ✅ All node info displays correctly
- ✅ Status shows "Connected"

---

### Analytics Tests ✅

#### Test 1: Difficulty Trend
1. Click **Analytics** tab
2. View "Difficulty Trend" chart
3. Mine several blocks
4. Observe chart updating

**Expected Result**:
- ✅ Chart displays with historical data
- ✅ Updates as new blocks added
- ✅ Line shows trend

#### Test 2: Transaction Activity
1. View "Transaction Activity" chart
2. Send a transaction
3. Mine block containing it
4. Observe chart update

**Expected Result**:
- ✅ Bar chart shows transaction count per block
- ✅ Updates with new blocks

#### Test 3: Block Rewards
1. View "Block Rewards" chart
2. Mine several blocks
3. Observe reward tracking

**Expected Result**:
- ✅ Line chart shows reward values
- ✅ Reflects current block rewards

---

### Block Explorer Tests ✅

#### Test 1: Search Blocks
1. Click **Block Explorer** tab
2. Enter block height in search
3. Verify blocks filtered

**Expected Result**:
- ✅ Search filters blocks
- ✅ Shows matching blocks only

#### Test 2: View Block Details
1. Click on a block to expand
2. Verify showing:
   - Height
   - Timestamp
   - Difficulty
   - Nonce
   - Reward
   - Hash details

**Expected Result**:
- ✅ Block expands with details
- ✅ All fields populated
- ✅ Hashes formatted correctly

#### Test 3: View Block Transactions
1. Expand a block with transactions
2. Verify transaction list shows
3. Click copy button on transaction hash

**Expected Result**:
- ✅ Transactions display in block
- ✅ Hash can be copied

#### Test 4: Block Navigation
1. Click multiple blocks
2. Verify expand/collapse works smoothly

**Expected Result**:
- ✅ Smooth animation
- ✅ Details display correctly

---

### Dashboard Tab Tests ✅

#### Test 1: Key Statistics
1. Click **Dashboard** tab
2. Verify main stat cards:
   - Chain Height
   - Hashrate
   - Total Earned
   - Block Time

**Expected Result**:
- ✅ All cards populate
- ✅ Numbers update

#### Test 2: Supply Tracking
1. View "Token Supply" section
2. Verify showing:
   - Current Supply
   - Max Supply (420M)
   - Progress bar
   - Breakdown stats

**Expected Result**:
- ✅ Supply data displays
- ✅ Progress bar shows percentage

#### Test 3: Halving Schedule
1. View "Halving Schedule" section
2. Verify showing:
   - Current Era
   - Current Reward
   - Blocks until halving
   - Next block number
   - Next reward

**Expected Result**:
- ✅ Halving data displays
- ✅ Calculations correct

#### Test 4: Recent Blocks
1. View "Latest Blocks" section
2. Verify showing last 10 blocks
3. Mine new block
4. Observe new block at top

**Expected Result**:
- ✅ Block list updates
- ✅ Most recent first
- ✅ Difficulty and rewards show

---

## Performance Testing

### Test 1: Responsiveness
1. Open Developer Tools (F12)
2. Go to Network tab
3. Perform dashboard operations
4. Check response times

**Expected Result**:
- ✅ API responses < 500ms
- ✅ Page loads < 2s
- ✅ Smooth interactions

### Test 2: Memory Usage
1. Open DevTools → Memory tab
2. Take heap snapshot
3. Perform operations for 5 minutes
4. Take another snapshot
5. Compare sizes

**Expected Result**:
- ✅ No memory leaks
- ✅ Stable memory usage
- ✅ < 50MB for 5min usage

### Test 3: CPU Usage
1. Start mining
2. Open DevTools → Performance tab
3. Record performance
4. Check CPU usage

**Expected Result**:
- ✅ Dashboard CPU < 10% while mining
- ✅ Mining runs on backend (not frontend)

---

## Error Handling Tests

### Test 1: Wrong Node URL
1. Click **Settings**
2. Change Node URL to invalid address
3. Try to access dashboard

**Expected Result**:
- ✅ Error message appears
- ✅ "Connection Error" shown
- ✅ Can correct URL and retry

### Test 2: Invalid Address
1. Go to **Transactions** → **Send**
2. Enter invalid recipient address
3. Try to send

**Expected Result**:
- ✅ Validation error shown
- ✅ Transaction not sent
- ✅ Clear error message

### Test 3: Insufficient Balance
1. Try to send more than balance
2. Click **Send**

**Expected Result**:
- ✅ "Insufficient balance" error
- ✅ Transaction blocked

### Test 4: Empty Required Fields
1. Try to send without address/amount
2. Click **Send**

**Expected Result**:
- ✅ Validation errors for missing fields
- ✅ Cannot submit

---

## Cross-Browser Testing

Test dashboard in multiple browsers:

- [ ] Chrome/Chromium
- [ ] Firefox
- [ ] Safari
- [ ] Edge

**Expected Result**:
- ✅ All features work in all browsers
- ✅ Responsive design works
- ✅ localStorage works
- ✅ No console errors

---

## Mobile/Responsive Testing

1. Open dashboard on mobile device OR
2. Use Chrome DevTools device emulation

**Test Cases**:
- [ ] Portrait mode navigation
- [ ] Landscape mode charts
- [ ] Touch interactions
- [ ] Button sizes
- [ ] Text readability

**Expected Result**:
- ✅ All features accessible on mobile
- ✅ Responsive layout
- ✅ Touch-friendly buttons
- ✅ No horizontal scroll

---

## Stress Testing

### Test: Rapid Operations
1. Mine multiple blocks rapidly
2. Send multiple transactions
3. Check dashboard stability

**Expected Result**:
- ✅ No crashes
- ✅ Stats update correctly
- ✅ UI remains responsive

### Test: Long-Running
1. Leave mining running overnight
2. Observe stability
3. Check error logs

**Expected Result**:
- ✅ Mining continues
- ✅ Stats update correctly
- ✅ No memory leaks

---

## Test Report Template

```
Dashboard Test Report
====================

Date: [DATE]
Tester: [NAME]
Browser: [BROWSER] [VERSION]
OS: [OS]

Test Results:
- [ ] Wallet Management: PASS/FAIL
- [ ] Transactions: PASS/FAIL
- [ ] Mining: PASS/FAIL
- [ ] Network: PASS/FAIL
- [ ] Analytics: PASS/FAIL
- [ ] Explorer: PASS/FAIL
- [ ] Dashboard: PASS/FAIL
- [ ] Error Handling: PASS/FAIL
- [ ] Performance: PASS/FAIL

Issues Found:
1. [Description of any issues]

Notes:
[Additional notes]
```

---

## Automated Testing (Optional)

For advanced testing, consider:
- Cypress for E2E testing
- Jest for unit testing
- Lighthouse for performance
- Accessibility checkers

---

## Deployment Testing Checklist

Before deploying to production:

- [ ] All features tested locally
- [ ] No console errors
- [ ] Mobile responsive verified
- [ ] Performance acceptable
- [ ] Error handling working
- [ ] Documentation complete
- [ ] Security reviewed
- [ ] API endpoints verified
- [ ] Environment variables set
- [ ] CORS configured properly

---

**Status**: Ready for Testing
**Last Updated**: December 2025
