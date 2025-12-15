// TrinityChain Dashboard - Browser Console Diagnostic Test
// Copy and paste this into the browser console (F12) to test connectivity

console.log("%c🔍 TrinityChain Dashboard Diagnostics", "color: #0099ff; font-size: 16px; font-weight: bold");
console.log("Time:", new Date().toISOString());

// Test 1: Check localStorage
console.log("\n%c📦 Test 1: Browser Storage", "color: #00cc00; font-weight: bold");
try {
    const testKey = 'trinity_test_' + Date.now();
    localStorage.setItem(testKey, 'test');
    const value = localStorage.getItem(testKey);
    localStorage.removeItem(testKey);
    console.log("✓ localStorage working");
} catch (e) {
    console.error("✗ localStorage error:", e.message);
}

// Test 2: Fetch API configuration
console.log("\n%c🌐 Test 2: API Endpoint", "color: #00cc00; font-weight: bold");
const nodeUrl = window.location.hostname === 'localhost' ? 'http://localhost:3000' : '';
console.log("Node URL:", nodeUrl || "Using relative URL");

// Test 3: Test blockchain/stats endpoint
console.log("\n%c📊 Test 3: Blockchain Stats", "color: #00cc00; font-weight: bold");
fetch(`${nodeUrl}/api/blockchain/stats`)
    .then(res => {
        console.log("Response status:", res.status);
        console.log("Response type:", res.headers.get('content-type'));
        return res.json();
    })
    .then(data => {
        console.log("✓ Stats loaded:", data);
    })
    .catch(err => {
        console.error("✗ Stats error:", err.message);
    });

// Test 4: Test wallet creation
console.log("\n%c🔑 Test 4: Wallet Creation", "color: #00cc00; font-weight: bold");
fetch(`${nodeUrl}/api/wallet/create`, { method: 'POST' })
    .then(res => {
        console.log("Response status:", res.status);
        return res.json();
    })
    .then(data => {
        console.log("✓ Wallet created:", {
            address: data.address?.slice(0, 20) + '...',
            has_public_key: !!data.public_key
        });
    })
    .catch(err => {
        console.error("✗ Wallet error:", err.message);
    });

// Test 5: Test mining status
console.log("\n%c⚡ Test 5: Mining Status", "color: #00cc00; font-weight: bold");
fetch(`${nodeUrl}/api/mining/status`)
    .then(res => res.json())
    .then(data => {
        console.log("✓ Mining status:", data);
    })
    .catch(err => {
        console.error("✗ Mining error:", err.message);
    });

// Test 6: Test blockchain height
console.log("\n%c📈 Test 6: Blockchain Height", "color: #00cc00; font-weight: bold");
fetch(`${nodeUrl}/api/blockchain/height`)
    .then(res => res.json())
    .then(data => {
        console.log("✓ Chain height:", data);
    })
    .catch(err => {
        console.error("✗ Height error:", err.message);
    });

// Test 7: React Component Check
console.log("\n%c⚛️ Test 7: React Environment", "color: #00cc00; font-weight: bold");
console.log("React DevTools available:", !!window.__REACT_DEVTOOLS_GLOBAL_HOOK__);
console.log("Window location:", window.location.href);
console.log("User agent:", navigator.userAgent);

// Test 8: Dashboard Component State
console.log("\n%c🎛️ Test 8: Dashboard State", "color: #00cc00; font-weight: bold");
console.log("Document ready state:", document.readyState);
console.log("DOM elements loaded:", document.querySelectorAll('[role="button"]').length);

console.log("\n%c✅ Diagnostic tests complete", "color: #00ff00; font-size: 14px; font-weight: bold");
console.log("Check the Network tab (F12 → Network) to see API requests");
