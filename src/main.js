/* ==========================================================================
   CrumusiX Central Frontend Coordinator — Native Tauri & Dual-Engine Player
   ========================================================================== */

// Import Tauri APIs from the pre-injected global object
const { invoke } = window.__TAURI__.core;

// Premium Dialog Helper Functions (replaces default browser blocking modals)
window.alert = function(message, title = 'Notification') {
  return new Promise((resolve) => {
    const overlay = document.getElementById('custom-modal-overlay');
    const titleEl = document.getElementById('modal-title');
    const messageEl = document.getElementById('modal-message');
    const inputContainer = document.getElementById('modal-input-container');
    const btnCancel = document.getElementById('modal-btn-cancel');
    const btnOk = document.getElementById('modal-btn-ok');

    titleEl.textContent = title;
    messageEl.textContent = message;
    inputContainer.style.display = 'none';
    btnCancel.style.display = 'none';
    btnOk.style.display = 'block';

    const handleOk = () => {
      cleanup();
      resolve();
    };

    const cleanup = () => {
      overlay.classList.remove('active');
      btnOk.removeEventListener('click', handleOk);
      document.removeEventListener('keydown', handleKey);
    };

    const handleKey = (e) => {
      if (e.key === 'Enter' || e.key === 'Escape') {
        e.preventDefault();
        handleOk();
      }
    };

    btnOk.addEventListener('click', handleOk);
    document.addEventListener('keydown', handleKey);
    overlay.classList.add('active');
    btnOk.focus();
  });
};

window.confirm = function(message, title = 'Confirm') {
  return new Promise((resolve) => {
    const overlay = document.getElementById('custom-modal-overlay');
    const titleEl = document.getElementById('modal-title');
    const messageEl = document.getElementById('modal-message');
    const inputContainer = document.getElementById('modal-input-container');
    const btnCancel = document.getElementById('modal-btn-cancel');
    const btnOk = document.getElementById('modal-btn-ok');

    titleEl.textContent = title;
    messageEl.textContent = message;
    inputContainer.style.display = 'none';
    btnCancel.style.display = 'block';
    btnOk.style.display = 'block';

    const handleOk = () => {
      cleanup();
      resolve(true);
    };

    const handleCancel = () => {
      cleanup();
      resolve(false);
    };

    const cleanup = () => {
      overlay.classList.remove('active');
      btnOk.removeEventListener('click', handleOk);
      btnCancel.removeEventListener('click', handleCancel);
      document.removeEventListener('keydown', handleKey);
    };

    const handleKey = (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleOk();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        handleCancel();
      }
    };

    btnOk.addEventListener('click', handleOk);
    btnCancel.addEventListener('click', handleCancel);
    document.addEventListener('keydown', handleKey);
    overlay.classList.add('active');
    btnOk.focus();
  });
};

window.prompt = function(message, defaultValue = '', title = 'Input Required') {
  return new Promise((resolve) => {
    const overlay = document.getElementById('custom-modal-overlay');
    const titleEl = document.getElementById('modal-title');
    const messageEl = document.getElementById('modal-message');
    const inputContainer = document.getElementById('modal-input-container');
    const inputEl = document.getElementById('modal-input');
    const btnCancel = document.getElementById('modal-btn-cancel');
    const btnOk = document.getElementById('modal-btn-ok');

    titleEl.textContent = title;
    messageEl.textContent = message;
    inputContainer.style.display = 'block';
    inputEl.value = defaultValue;
    btnCancel.style.display = 'block';
    btnOk.style.display = 'block';

    const handleOk = () => {
      const val = inputEl.value;
      cleanup();
      resolve(val);
    };

    const handleCancel = () => {
      cleanup();
      resolve(null);
    };

    const cleanup = () => {
      overlay.classList.remove('active');
      btnOk.removeEventListener('click', handleOk);
      btnCancel.removeEventListener('click', handleCancel);
      inputEl.removeEventListener('keydown', handleInputKey);
      document.removeEventListener('keydown', handleKey);
    };

    const handleInputKey = (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleOk();
      }
    };

    const handleKey = (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        handleCancel();
      }
    };

    btnOk.addEventListener('click', handleOk);
    btnCancel.addEventListener('click', handleCancel);
    inputEl.addEventListener('keydown', handleInputKey);
    document.addEventListener('keydown', handleKey);
    overlay.classList.add('active');
    setTimeout(() => {
      inputEl.focus();
      inputEl.select();
    }, 50);
  });
};

// App Global States
let appState = {
  activeScreen: 'home',
  isPlaying: false,
  currentTrack: null, // { id, title, artist, album, duration, source, thumbnail }
  volume: 80,
  isMuted: false,
  repeatMode: 'off', // 'off' | 'track' | 'context'
  isShuffle: false,
  isAuthenticating: false,
  isMiniPlayer: false,
  currentLyrics: null,
  lyricOffsetMs: 0,
  prefetchedNextLyrics: false,
  showVideo: true,

  // Spotify Auth Session
  spotifyToken: null,
  spotifyRefreshToken: null,
  spotifyTokenExpiry: 0,
  spotifyClientId: 'd08327e1f5a14ea59b1feda104f5c255',
  spotifyDeviceId: null,

  // Spotify Native Audio Engine settings
  spotifySettings: {
    normalisation: true,
    cacheEnabled: true,
    gapless: true,
    mprisEnabled: true,
    cacheSizeMb: 2000,
  },

  // Performance and Graphics optimization settings
  performanceSettings: {
    visualizerEnabled: false,
    premiumGraphics: true,
  },

  // Playlists and Queue
  playlists: [],
  activePlaylist: null,
  queue: [],
  queueIndex: -1,
  recentTracks: [],
};

function syncConfigToAppState(config) {
  appState.activeScreen = config.active_screen;
  appState.volume = config.volume;
  appState.isMuted = config.is_muted;
  appState.spotifyToken = config.spotify_access_token;
  appState.spotifyRefreshToken = config.spotify_refresh_token;
  appState.spotifyTokenExpiry = config.spotify_token_expiry;
  appState.spotifyClientId = config.spotify_client_id;
  appState.showVideo = config.show_video;
  appState.spotifySettings = {
    normalisation: config.spotify_settings.normalisation,
    cacheEnabled: config.spotify_settings.cache_enabled,
    gapless: config.spotify_settings.gapless,
    mprisEnabled: config.spotify_settings.mpris_enabled,
    cacheSizeMb: config.spotify_settings.cache_size_mb,
  };
  appState.performanceSettings = {
    visualizerEnabled: config.performance_settings.visualizer_enabled,
    premiumGraphics: config.performance_settings.premium_graphics,
  };
  appState.recentTracks = config.recent_tracks;
}

async function saveConfig() {
  const config = {
    active_screen: appState.activeScreen,
    volume: appState.volume,
    is_muted: appState.isMuted,
    spotify_access_token: appState.spotifyToken,
    spotify_refresh_token: appState.spotifyRefreshToken,
    spotify_token_expiry: appState.spotifyTokenExpiry,
    spotify_client_id: appState.spotifyClientId,
    show_video: appState.showVideo,
    spotify_settings: {
      normalisation: appState.spotifySettings.normalisation,
      cache_enabled: appState.spotifySettings.cacheEnabled,
      gapless: appState.spotifySettings.gapless,
      mpris_enabled: appState.spotifySettings.mprisEnabled,
      cache_size_mb: appState.spotifySettings.cacheSizeMb,
    },
    performance_settings: {
      visualizer_enabled: appState.performanceSettings.visualizerEnabled,
      premium_graphics: appState.performanceSettings.premiumGraphics,
    },
    recent_tracks: appState.recentTracks,
  };
  await invoke('save_app_config', { config }).catch(console.error);
}

// Playback Player References
let ytPlayer = null;
let timelineInterval = null;

// Search & Debounce Variables
let searchDebounceTimeout = null;
let activeSearchQuery = '';

// ==========================================================================
// 1. Initializers & Listeners
// ==========================================================================
window.addEventListener('DOMContentLoaded', async () => {
  try {
    const config = await invoke('get_app_config');
    syncConfigToAppState(config);
  } catch (e) {
    console.error("Failed to load app config from Rust:", e);
  }

  initUI();
  initOAuthReceiver();
  loadPlaylists();
  renderRecentTracks();
  initNativeSpotifyEvents();
  initNativeQueueState();
  initLocalLibrary();
  startDiagnosticsPolling();

  // Apply saved volume and settings
  document.getElementById('player-volume-slider').value = appState.volume;
  updateVolumeFill(appState.volume);
  switchScreen(appState.activeScreen);
});

// ==========================================================================
// 2. UI View Transitions & Sidebar Screen Routing
// ==========================================================================
function initUI() {
  // Sidebar buttons navigation listener
  document.querySelectorAll('.nav-item').forEach(button => {
    button.addEventListener('click', (e) => {
      const screenId = button.getAttribute('data-screen');
      switchScreen(screenId);
    });
  });

  // Welcome triggers
  document.querySelectorAll('.spotify-connect-trigger').forEach(el => {
    el.addEventListener('click', () => switchScreen('spotify-connect'));
  });
  document.querySelectorAll('.go-search-trigger').forEach(el => {
    el.addEventListener('click', () => switchScreen('search'));
  });



  // Native Spotify Audio Engine Settings UI setup
  const normCheckbox = document.getElementById('setting-spotify-normalisation');
  const cacheCheckbox = document.getElementById('setting-spotify-cache');
  const sizeSelect = document.getElementById('setting-spotify-cache-size');
  const gaplessCheckbox = document.getElementById('setting-spotify-gapless');
  const mprisCheckbox = document.getElementById('setting-spotify-mpris');

  if (normCheckbox) {
    normCheckbox.checked = appState.spotifySettings.normalisation;
    normCheckbox.addEventListener('change', (e) => {
      appState.spotifySettings.normalisation = e.target.checked;
      saveConfig();
      syncSpotifySettings();
    });
  }

  if (cacheCheckbox) {
    cacheCheckbox.checked = appState.spotifySettings.cacheEnabled;
    cacheCheckbox.addEventListener('change', (e) => {
      appState.spotifySettings.cacheEnabled = e.target.checked;
      saveConfig();
      const sizeContainer = document.getElementById('cache-size-container');
      if (sizeContainer) {
        sizeContainer.style.display = e.target.checked ? 'flex' : 'none';
      }
      syncSpotifySettings();
    });
    const sizeContainer = document.getElementById('cache-size-container');
    if (sizeContainer) {
      sizeContainer.style.display = appState.spotifySettings.cacheEnabled ? 'flex' : 'none';
    }
  }

  if (sizeSelect) {
    sizeSelect.value = appState.spotifySettings.cacheSizeMb;
    sizeSelect.addEventListener('change', (e) => {
      appState.spotifySettings.cacheSizeMb = parseInt(e.target.value);
      saveConfig();
      syncSpotifySettings();
    });
  }

  const showVideoCheckbox = document.getElementById('setting-show-video');
  if (showVideoCheckbox) {
    showVideoCheckbox.checked = appState.showVideo;
    showVideoCheckbox.addEventListener('change', (e) => {
      appState.showVideo = e.target.checked;
      saveConfig();
    });
  }

  if (gaplessCheckbox) {
    gaplessCheckbox.checked = appState.spotifySettings.gapless;
    gaplessCheckbox.addEventListener('change', (e) => {
      appState.spotifySettings.gapless = e.target.checked;
      saveConfig();
      syncSpotifySettings();
    });
  }

  if (mprisCheckbox) {
    mprisCheckbox.checked = appState.spotifySettings.mprisEnabled;
    mprisCheckbox.addEventListener('change', (e) => {
      appState.spotifySettings.mprisEnabled = e.target.checked;
      saveConfig();
      syncSpotifySettings();
    });
  }

  // Resource & Graphics Optimization Settings UI setup
  const perfGraphicsCheckbox = document.getElementById('setting-performance-graphics');

  function applyGraphicsMode() {
    if (appState.performanceSettings.premiumGraphics) {
      document.body.classList.remove('low-spec-mode');
    } else {
      document.body.classList.add('low-spec-mode');
    }
  }

  if (perfGraphicsCheckbox) {
    perfGraphicsCheckbox.checked = appState.performanceSettings.premiumGraphics;
    applyGraphicsMode();
    perfGraphicsCheckbox.addEventListener('change', (e) => {
      appState.performanceSettings.premiumGraphics = e.target.checked;
      saveConfig();
      applyGraphicsMode();
    });
  } else {
    applyGraphicsMode();
  }

  // Search input listeners
  const searchInput = document.getElementById('search-input');
  const searchClearBtn = document.getElementById('search-clear-btn');

  searchInput.addEventListener('input', (e) => {
    const val = e.target.value.trim();
    searchClearBtn.style.display = val ? 'block' : 'none';

    clearTimeout(searchDebounceTimeout);

    if (val.length === 0) {
      clearSearchResults();
      return;
    }

    if (val.length >= 2) {
      searchDebounceTimeout = setTimeout(() => {
        performSearch(val);
      }, 400); // 400ms debounce waiting time
    }
  });

  searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const val = searchInput.value.trim();
      if (val.length >= 2) {
        clearTimeout(searchDebounceTimeout);
        performSearch(val);
      }
    }
  });

  searchClearBtn.addEventListener('click', () => {
    searchInput.value = '';
    searchClearBtn.style.display = 'none';
    clearSearchResults();
  });

  // Search tab selectors
  document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const tabId = btn.getAttribute('data-tab');
      document.getElementById('tab-' + tabId).classList.add('active');
    });
  });

  // Sync seek and volume controls
  document.getElementById('player-seek-slider').addEventListener('input', (e) => {
    const val = parseFloat(e.target.value);
    document.getElementById('player-seek-fill').style.width = val + '%';
    seekPlayback(val);
  });

  document.getElementById('player-volume-slider').addEventListener('input', (e) => {
    const val = parseInt(e.target.value);
    appState.volume = val;
    saveConfig();
    updateVolumeFill(val);
    setVolume(val);
  });

  document.getElementById('ctrl-volume-mute').addEventListener('click', () => {
    appState.isMuted = !appState.isMuted;
    saveConfig();
    const icon = document.querySelector('#ctrl-volume-mute svg');
    if (appState.isMuted) {
      icon.style.opacity = '0.4';
      setVolume(0);
    } else {
      icon.style.opacity = '1';
      setVolume(appState.volume);
    }
  });

  // Playback control buttons
  document.getElementById('ctrl-play-pause').addEventListener('click', togglePlayPause);
  document.getElementById('ctrl-next').addEventListener('click', playNext);
  document.getElementById('ctrl-prev').addEventListener('click', playPrev);

  document.getElementById('ctrl-shuffle').addEventListener('click', () => {
    const target = !appState.isShuffle;
    invoke('queue_set_shuffle', { shuffle: target }).catch(console.error);
  });

  document.getElementById('ctrl-repeat').addEventListener('click', () => {
    let target = 'off';
    if (appState.repeatMode === 'off') {
      target = 'track';
    } else if (appState.repeatMode === 'track') {
      target = 'queue';
    }
    invoke('queue_set_repeat_mode', { mode: target }).catch(console.error);
  });

  // Likes trigger
  document.getElementById('player-like-btn').addEventListener('click', () => {
    if (!appState.currentTrack) return;
    document.getElementById('player-like-btn').classList.toggle('liked');
  });

  // Spotify Connect Authorization Flow
  document.getElementById('btn-spotify-login').addEventListener('click', startSpotifyLoginFlow);

  // Playlists manager trigger
  document.getElementById('btn-new-playlist').addEventListener('click', createNewPlaylist);

  // Compact Video corner controls
  document.getElementById('close-video-corner').addEventListener('click', () => {
    document.getElementById('compact-video-view').classList.add('video-corner-hidden');
  });

  document.getElementById('btn-toggle-mini-video').addEventListener('click', () => {
    const videoCorner = document.getElementById('compact-video-view');
    videoCorner.classList.toggle('video-corner-hidden');
  });

  // Custom styled window title bar buttons
  if (window.__TAURI__ && window.__TAURI__.window && typeof window.__TAURI__.window.getCurrentWindow === 'function') {
    const appWindow = window.__TAURI__.window.getCurrentWindow();

    // Fallback/robust dragging support for Linux / custom titlebars via JS startDragging API
    const titlebar = document.querySelector('.window-titlebar');
    if (titlebar) {
      titlebar.addEventListener('mousedown', (e) => {
        // Only start dragging on left click, and make sure we aren't clicking a button or control
        if (e.button === 0 && !e.target.closest('.window-controls') && !e.target.closest('button') && !e.target.closest('a') && !e.target.closest('input')) {
          appWindow.startDragging().catch(console.error);
        }
      });
    }

    document.getElementById('win-minimize').addEventListener('click', () => {
      appWindow.minimize().catch(console.error);
    });
    document.getElementById('win-maximize').addEventListener('click', () => {
      appWindow.isMaximized().then(maximized => {
        if (maximized) {
          appWindow.unmaximize().catch(console.error);
        } else {
          appWindow.maximize().catch(console.error);
        }
      }).catch(() => {
        appWindow.maximize().catch(console.error);
      });
    });
    document.getElementById('win-close').addEventListener('click', () => {
      appWindow.close().catch(console.error);
    });
  } else {
    document.getElementById('win-minimize').addEventListener('click', () => {
      console.warn("Tauri Window API is not available.");
    });
    document.getElementById('win-maximize').addEventListener('click', () => {
      console.warn("Tauri Window API is not available.");
    });
    document.getElementById('win-close').addEventListener('click', () => {
      console.warn("Tauri Window API is not available.");
    });
  }


  // Diagnostics trigger
  document.getElementById('btn-run-diagnostics').addEventListener('click', runSystemDiagnostics);

  // Play Queue Toggle Drawer bind
  const toggleQueueBtn = document.getElementById('btn-toggle-queue');
  const queueSidebar = document.querySelector('.app-queue-sidebar');
  if (toggleQueueBtn && queueSidebar) {
    toggleQueueBtn.addEventListener('click', () => {
      queueSidebar.classList.toggle('queue-sidebar-hidden');
      toggleQueueBtn.classList.toggle('active', !queueSidebar.classList.contains('queue-sidebar-hidden'));
    });
  }

  // Clear Queue bind
  const clearQueueBtn = document.getElementById('btn-queue-clear');
  if (clearQueueBtn) {
    clearQueueBtn.addEventListener('click', async () => {
      if (await confirm("Are you sure you want to clear the play queue?")) {
        invoke('queue_clear').catch(console.error);
      }
    });
  }

  // Shuffle Queue bind
  const shuffleQueueBtn = document.getElementById('btn-queue-shuffle');
  if (shuffleQueueBtn) {
    shuffleQueueBtn.addEventListener('click', () => {
      invoke('queue_shuffle').catch(console.error);
    });
  }
}

function switchScreen(screenId) {
  document.querySelectorAll('.display-screen').forEach(screen => {
    screen.classList.remove('active');
  });
  document.querySelectorAll('.nav-item').forEach(btn => {
    btn.classList.remove('active');
    if (btn.getAttribute('data-screen') === screenId) {
      btn.classList.add('active');
    }
  });

  const target = document.getElementById('screen-' + screenId);
  if (target) {
    target.classList.add('active');
    appState.activeScreen = screenId;
    saveConfig();
  }

  // Refresh Spotify auth UI details on entering auth screen
  if (screenId === 'spotify-connect') {
    updateSpotifyAuthScreen();
  }

  // Refresh Lyrics Cache Statistics on entering settings screen
  if (screenId === 'settings') {
    updateLyricsCacheStats();
  }

  // Load offline library tracks on entering library screen
  if (screenId === 'library') {
    loadLibraryCatalog();
  }
}
window.switchScreen = switchScreen;

function updateVolumeFill(val) {
  document.getElementById('player-volume-fill').style.width = val + '%';
}

// ==========================================================================
// 3. Spotify PKCE OAuth Authentication Flows
// ==========================================================================
const REDIRECT_URI = 'http://127.0.0.1:8888';

async function startSpotifyLoginFlow() {
  if (appState.isAuthenticating) {
    console.warn("Spotify authentication is already in progress.");
    return;
  }

  const cid = appState.spotifyClientId ? appState.spotifyClientId.trim() : '';
  if (!cid) {
    await alert("Please go to the 'Settings' tab first, paste your custom Spotify Client ID, and try connecting again!");
    switchScreen('settings');
    return;
  }

  const loginBtn = document.getElementById('btn-spotify-login');
  if (loginBtn) {
    loginBtn.disabled = true;
    loginBtn.textContent = "Awaiting Spotify Login...";
    loginBtn.style.opacity = "0.6";
    loginBtn.style.cursor = "not-allowed";
  }
  appState.isAuthenticating = true;

  try {
    // 1. Let Rust PKCE engine construct the auth url
    const authUrl = await invoke('spotify_generate_auth_url', { clientId: cid });

    // Helper to safely reset authenticating state and button
    const finishAuthFlow = () => {
      appState.isAuthenticating = false;
      if (loginBtn) {
        loginBtn.disabled = false;
        loginBtn.textContent = "Connect Spotify Account";
        loginBtn.style.opacity = "";
        loginBtn.style.cursor = "";
      }
    };

    // 2. Fire up the native Rust loopback server asynchronously to capture the incoming authorization code
    invoke('start_oauth_server')
      .then(async (code) => {
        console.log('Successfully captured Spotify authorization callback.');
        const newSession = await invoke('spotify_exchange_code', { code: code, clientId: cid });
        appState.spotifyToken = newSession.access_token;
        appState.spotifyRefreshToken = newSession.refresh_token;
        appState.spotifyTokenExpiry = newSession.expires_at;
        await saveConfig();
        updateSpotifyAuthScreen();
        initNativeSpotify();
        switchScreen('spotify-connect');
      })
      .catch(async (err) => {
        console.error('Local OAuth server failed:', err);
        await alert('Authentication failed to start local loopback listener: ' + err);
      })
      .finally(() => {
        finishAuthFlow();
      });

    // 3. Open the auth URL in the system browser via the Rust opener plugin.
    //    This is more reliable than a WebView2 popup window, especially on Windows.
    invoke('open_auth_window', { url: authUrl })
      .catch(err => {
        console.error('Failed to open auth URL via Rust opener, trying window.open:', err);
        // Direct fallback: open in the default browser via JavaScript.
        // Tauri v2 webviews allow window.open to external URLs.
        window.open(authUrl, '_blank');
      });
  } catch (err) {
    console.error('Spotify login flow initiation failed:', err);
    await alert('Failed to initiate login flow: ' + err);
    appState.isAuthenticating = false;
    if (loginBtn) {
      loginBtn.disabled = false;
      loginBtn.textContent = "Connect Spotify Account";
      loginBtn.style.opacity = "";
      loginBtn.style.cursor = "";
    }
  }
}

async function initOAuthReceiver() {
  if (appState.spotifyToken) {
    // Inject the active credentials into the Rust shared session on startup
    const session = {
      access_token: appState.spotifyToken,
      refresh_token: appState.spotifyRefreshToken,
      expires_at: appState.spotifyTokenExpiry,
      client_id: appState.spotifyClientId,
    };
    await invoke('spotify_inject_session', { session }).catch(console.error);

    // Call spotify_get_session to verify/refresh token status natively
    try {
      const activeSession = await invoke('spotify_get_session');
      appState.spotifyToken = activeSession.access_token;
      appState.spotifyRefreshToken = activeSession.refresh_token;
      appState.spotifyTokenExpiry = activeSession.expires_at;
      await saveConfig();
      initNativeSpotify();
    } catch (err) {
      console.error('Failed to restore Spotify session:', err);
      logoutSpotify();
    }
  } else {
    // Check if backend has cached credentials and auto-initialize
    setTimeout(async () => {
      try {
        const state = await invoke('spotify_get_state');
        if (state.is_connected) {
          appState.spotifyDeviceId = "native-rust-engine";
          updateSpotifyAuthScreen();
        }
      } catch (e) { }
    }, 1000);
  }
}

async function refreshSpotifyAccessToken() {
  try {
    const activeSession = await invoke('spotify_get_session');
    appState.spotifyToken = activeSession.access_token;
    appState.spotifyRefreshToken = activeSession.refresh_token;
    appState.spotifyTokenExpiry = activeSession.expires_at;
    await saveConfig();
    console.log("Spotify access token refreshed successfully.");
  } catch (err) {
    console.error("Failed to refresh Spotify token:", err);
    throw err;
  }
}

async function initNativeSpotify() {
  try {
    const settings = {
      normalisation: appState.spotifySettings.normalisation,
      cache_enabled: appState.spotifySettings.cacheEnabled,
      gapless: appState.spotifySettings.gapless,
      mpris_enabled: appState.spotifySettings.mprisEnabled,
      cache_size_mb: appState.spotifySettings.cacheSizeMb,
    };
    await invoke('spotify_init_native', { token: appState.spotifyToken, settings });
    appState.spotifyDeviceId = "native-rust-engine";
    updateSpotifyAuthScreen();
  } catch (err) {
    console.error('Failed to initialize native Spotify backend:', err);
  }
}

async function syncSpotifySettings() {
  try {
    const settings = {
      normalisation: appState.spotifySettings.normalisation,
      cache_enabled: appState.spotifySettings.cacheEnabled,
      gapless: appState.spotifySettings.gapless,
      mpris_enabled: appState.spotifySettings.mprisEnabled,
      cache_size_mb: appState.spotifySettings.cacheSizeMb,
    };
    await invoke('spotify_update_settings', { settings });
    console.log('Spotify settings updated dynamically:', settings);
  } catch (err) {
    console.error('Failed to sync Spotify settings to backend:', err);
  }
}



async function logoutSpotify() {
  appState.spotifyToken = null;
  appState.spotifyRefreshToken = null;
  appState.spotifyDeviceId = null;
  appState.spotifyTokenExpiry = 0;

  await saveConfig();
  await invoke('spotify_logout_session').catch(console.error);
  invoke('spotify_stop').catch(console.error);

  updateSpotifyAuthScreen();
}

function updateSpotifyAuthScreen() {
  const spotifyBadge = document.querySelector('.spotify-badge');
  const loginWrapper = document.getElementById('spotify-auth-login-wrapper');
  const dash = document.getElementById('spotify-dashboard');

  if (appState.spotifyToken) {
    if (spotifyBadge) spotifyBadge.classList.add('connected');
    if (loginWrapper) loginWrapper.style.display = 'none';
    if (dash) {
      dash.style.display = 'block';
      // Always call loadSpotifyDashboard when dashboard is shown
      loadSpotifyDashboard();
    }
  } else {
    if (spotifyBadge) spotifyBadge.classList.remove('connected');
    if (loginWrapper) loginWrapper.style.display = 'flex';
    if (dash) dash.style.display = 'none';

    const statusCont = document.getElementById('spotify-auth-status');
    if (statusCont) {
      statusCont.innerHTML = `
        <span class="status-indicator disconnected">Disconnected</span>
        <p class="status-details">No active session found. Connect your Spotify Premium account below.</p>
      `;
    }
    const loginBtn = document.getElementById('btn-spotify-login');
    if (loginBtn) loginBtn.style.display = 'block';
  }
}

function parseCSV(text) {
  const lines = [];
  let row = [""];
  let inQuotes = false;

  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    const next = text[i + 1];

    if (c === '"') {
      if (inQuotes && next === '"') {
        row[row.length - 1] += '"';
        i++;
      } else {
        inQuotes = !inQuotes;
      }
    } else if (c === ',' && !inQuotes) {
      row.push('');
    } else if ((c === '\r' || c === '\n') && !inQuotes) {
      if (c === '\r' && next === '\n') {
        i++;
      }
      lines.push(row);
      row = [''];
    } else {
      row[row.length - 1] += c;
    }
  }
  if (row.length > 1 || row[0] !== '') {
    lines.push(row);
  }
  return lines;
}

function parseImportedData(rawContent) {
  rawContent = rawContent.trim();
  if (!rawContent) {
    throw new Error("Content is empty.");
  }

  // Check if it is JSON
  if (rawContent.startsWith('{') || rawContent.startsWith('[')) {
    const data = JSON.parse(rawContent);
    let items = [];
    if (Array.isArray(data)) {
      items = data;
    } else if (data.items && Array.isArray(data.items)) {
      items = data.items;
    } else {
      throw new Error("Could not find a list of tracks under 'items' or directly as root array in JSON.");
    }

    const tracks = [];
    items.forEach(item => {
      let track = item.track || item.item || item;
      if (!track) return;
      if (!track.name) return;

      const trackId = track.id || `local-import-${Math.random().toString(36).substr(2, 9)}`;
      const duration_ms = track.duration_ms || 0;
      const total_sec = Math.floor(duration_ms / 1000);
      const min = Math.floor(total_sec / 60);
      const sec = total_sec % 60;
      const dur_str = `${min}:${sec < 10 ? '0' : ''}${sec}`;

      const artist = Array.isArray(track.artists)
        ? track.artists.map(a => a.name).join(", ")
        : (track.artist || "Unknown Artist");

      const album_name = (track.album && track.album.name)
        ? track.album.name
        : (track.album || "Unknown Album");

      let thumbnail = "";
      if (track.album && Array.isArray(track.album.images) && track.album.images.length > 0) {
        thumbnail = track.album.images[0].url;
      } else if (track.thumbnail) {
        thumbnail = track.thumbnail;
      }

      tracks.push({
        id: trackId,
        title: track.name,
        artist: artist,
        album: album_name,
        duration: dur_str,
        source: "spotify",
        thumbnail: thumbnail
      });
    });
    return tracks;
  }

  // Parse as CSV
  const csvRows = parseCSV(rawContent);
  if (csvRows.length < 2) {
    throw new Error("CSV does not contain header and data lines.");
  }

  const headers = csvRows[0].map(h => h.trim().toLowerCase());
  
  const uriIdx = headers.findIndex(h => h.includes("track uri") || (h.includes("uri") && !h.includes("artist") && !h.includes("album")));
  const nameIdx = headers.findIndex(h => (h.includes("name") || h.includes("title")) && !h.includes("artist") && !h.includes("album") && !h.includes("uri"));
  const artistIdx = headers.findIndex(h => h.includes("artist") && !h.includes("uri") && !h.includes("url"));
  const albumIdx = headers.findIndex(h => h.includes("album") && !h.includes("uri") && !h.includes("url"));
  const durationIdx = headers.findIndex(h => h.includes("duration"));
  const imageIdx = headers.findIndex(h => h.includes("image") || h.includes("artwork") || h.includes("thumbnail") || h.includes("pic"));

  if (nameIdx === -1) {
    throw new Error("Could not find track name column in CSV. Headers found: " + csvRows[0].join(", "));
  }

  const tracks = [];
  for (let r = 1; r < csvRows.length; r++) {
    const row = csvRows[r];
    if (row.length < headers.length && row.join("").trim() === "") continue; // Skip blank lines
    
    let trackId = uriIdx !== -1 ? (row[uriIdx] || "") : "";
    if (trackId.includes("spotify:track:")) {
      trackId = trackId.split("spotify:track:")[1];
    }
    if (!trackId) {
      trackId = `local-import-${Math.random().toString(36).substr(2, 9)}`;
    }

    const title = nameIdx !== -1 ? row[nameIdx] : "";
    if (!title) continue;

    const artist = artistIdx !== -1 ? row[artistIdx] : "Unknown Artist";
    const album = albumIdx !== -1 ? row[albumIdx] : "Unknown Album";

    let dur_str = "0:00";
    if (durationIdx !== -1 && row[durationIdx]) {
      const ms = parseInt(row[durationIdx]) || 0;
      const total_sec = Math.floor(ms / 1000);
      const min = Math.floor(total_sec / 60);
      const sec = total_sec % 60;
      dur_str = `${min}:${sec < 10 ? '0' : ''}${sec}`;
    }

    const thumbnail = imageIdx !== -1 ? (row[imageIdx] || "").trim() : "";

    tracks.push({
      id: trackId,
      title: title,
      artist: artist,
      album: album,
      duration: dur_str,
      source: "spotify",
      thumbnail: thumbnail
    });
  }

  return tracks;
}

function openSpotifyImportModal() {
  const modal = document.getElementById('spotify-import-modal');
  const closeBtn = document.getElementById('btn-close-spotify-import');
  const submitBtn = document.getElementById('btn-spotify-import-submit');
  const inputArea = document.getElementById('spotify-import-json-input');
  const errorDiv = document.getElementById('spotify-import-error');
  const fileInput = document.getElementById('spotify-import-csv-file');

  if (!modal || !closeBtn || !submitBtn || !inputArea || !errorDiv) return;

  errorDiv.style.display = 'none';
  inputArea.value = '';
  if (fileInput) fileInput.value = '';
  modal.classList.remove('palette-hidden');

  const closeModal = () => {
    modal.classList.add('palette-hidden');
  };

  closeBtn.onclick = closeModal;

  modal.onclick = (e) => {
    if (e.target === modal) {
      closeModal();
    }
  };

  if (fileInput) {
    fileInput.onchange = (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (evt) => {
        const text = evt.target.result;
        inputArea.value = text;
        errorDiv.style.display = 'none';
        
        try {
          const tracks = parseImportedData(text);
          localStorage.setItem('spotify_on_repeat_imported', JSON.stringify(tracks));
          closeModal();
          renderOnRepeatTracks(tracks, true);
          resolveSpotifyTrackArtworks(tracks);
        } catch (err) {
          errorDiv.textContent = `Error parsing file: ${err.message}`;
          errorDiv.style.display = 'block';
        }
      };
      reader.onerror = () => {
        errorDiv.textContent = "Failed to read file.";
        errorDiv.style.display = 'block';
      };
      reader.readAsText(file);
    };
  }

  submitBtn.onclick = () => {
    const rawContent = inputArea.value.trim();
    if (!rawContent) {
      errorDiv.textContent = "Please select a file or paste content first.";
      errorDiv.style.display = 'block';
      return;
    }

    try {
      const tracks = parseImportedData(rawContent);
      localStorage.setItem('spotify_on_repeat_imported', JSON.stringify(tracks));
      closeModal();
      renderOnRepeatTracks(tracks, true);
      resolveSpotifyTrackArtworks(tracks);
    } catch (err) {
      errorDiv.textContent = `Error parsing content: ${err.message}`;
      errorDiv.style.display = 'block';
    }
  };
}

async function resolveSpotifyTrackArtworks(tracks) {
  const tracksNeedingArt = tracks.filter(t => !t.thumbnail && t.id && !t.id.startsWith("local-import-"));
  if (tracksNeedingArt.length === 0) return;

  console.log(`[Spotify Art] Resolving artwork for ${tracksNeedingArt.length} imported tracks via public oEmbed fallback...`);
  
  for (const track of tracksNeedingArt) {
    try {
      const url = `https://open.spotify.com/oembed?url=spotify:track:${track.id}`;
      const response = await fetch(url);
      if (response.ok) {
        const data = await response.json();
        if (data.thumbnail_url) {
          track.thumbnail = data.thumbnail_url;
          // Render changes immediately as they resolve
          renderOnRepeatTracks(tracks, true);
        }
      }
      // Polite 50ms pause between requests to prevent rate limiting
      await new Promise(r => setTimeout(r, 50));
    } catch (e) {
      console.warn(`[Spotify Art] Failed to resolve artwork for track ${track.id}:`, e);
    }
  }

  localStorage.setItem('spotify_on_repeat_imported', JSON.stringify(tracks));
}

function renderOnRepeatTracks(tracks, isImported) {
  const onRepeatContainer = document.getElementById('spotify-on-repeat');
  if (!onRepeatContainer) return;
  onRepeatContainer.innerHTML = '';

  const badge = document.getElementById('on-repeat-source-badge');
  if (badge) {
    badge.style.display = 'inline-block';
    if (isImported) {
      badge.textContent = 'Imported';
      badge.style.background = 'rgba(168, 85, 247, 0.15)';
      badge.style.color = '#c084fc';
    } else {
      badge.textContent = 'Live';
      badge.style.background = 'rgba(29, 185, 84, 0.15)';
      badge.style.color = '#1DB954';
    }
  }

  if (tracks && tracks.length > 0) {
    tracks.forEach((track, idx) => {
      const row = createTrackRowHTML(track, idx + 1);
      onRepeatContainer.appendChild(row);
    });
  } else {
    onRepeatContainer.innerHTML = `
      <div style="padding:20px; color:var(--color-text-dim); text-align:center;">
        No tracks loaded. Click <strong>Import File</strong> above or upload your Exportify file:
        <br/>
        <button id="btn-inner-import-onrepeat" class="primary-btn" style="margin-top:12px; padding:6px 14px; font-size:0.8rem; background:var(--color-accent); color:white; border:none; border-radius:4px; font-weight:600; cursor:pointer;">Import Exportify CSV / JSON</button>
      </div>`;
    const innerBtn = document.getElementById('btn-inner-import-onrepeat');
    if (innerBtn) {
      innerBtn.addEventListener('click', openSpotifyImportModal);
    }
  }
}

async function loadSpotifyDashboard() {
  const container = document.getElementById('spotify-dashboard');
  if (!container) return;

  // Only render dashboard if we have a valid Spotify token
  if (!appState.spotifyToken) {
    container.style.display = 'none';
    return;
  }

  container.style.display = 'block';
  container.innerHTML = `
    <div class="spotify-dashboard-container">
      <!-- TOP: Connect / Session Section (Full Width) -->
      <div class="settings-group-card" style="margin-bottom: 20px; max-width: 100%;">
        <div class="group-header" style="display: flex; align-items: center; gap: 8px;">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="#1DB954">
            <path d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2ZM16.5858 16.4244C16.3902 16.7468 15.9686 16.8524 15.6462 16.6568C13.1118 15.1102 9.98705 14.7656 6.22123 15.626C5.85963 15.7088 5.49802 15.4828 5.41522 15.1212C5.33242 14.7596 5.55843 14.398 5.92003 14.3152C10.0306 13.3736 13.4862 13.7656 16.3534 15.51C16.6758 15.7056 16.7814 16.1272 16.5858 16.4244ZM17.9724 13.3888C17.7236 13.7958 17.1884 13.924 16.7814 13.6752C13.8884 11.899 9.47365 11.3862 6.09063 12.4116C5.63082 12.551 5.14842 12.295 5.00902 11.8352C4.86962 11.3754 5.12563 10.893 5.58544 10.7536C9.45825 9.57782 14.3254 10.1508 17.6724 12.2116C18.0646 12.4452 18.2078 12.9818 17.9724 13.3888ZM18.0854 10.2796C15.1182 8.51662 10.2132 8.35102 7.37324 9.21342C6.91343 9.35282 6.42363 9.08922 6.28423 8.62942C6.14483 8.16962 6.40843 7.67982 6.86823 7.54042C10.1382 6.54702 15.545 6.72782 18.9954 8.78462C19.41 9.03322 19.5456 9.57602 19.297 9.99042C19.0484 10.4048 18.5056 10.5404 18.0854 10.2796Z" />
          </svg>
          Spotify Account Integration
        </div>
        <div class="system-stats-card">
          <div class="stat-line">
            <span>Integration Status:</span>
            <strong style="color: var(--color-spotify);">Connected and Active</strong>
          </div>
          <button class="danger-outline-btn" id="btn-spotify-logout">Disconnect Spotify Account</button>
        </div>
        <p class="setting-hint">Your Spotify Premium account is authenticated. The native PKCE Librespot audio engine is active and ready.</p>
      </div>

      <!-- BOTTOM: Two-Column Grid -->
      <div class="spotify-dashboard-grid-2col">
        <!-- Left Column: Spotify Playlists -->
        <div class="settings-group-card" style="max-width: 100%;">
          <div class="group-header" style="display: flex; align-items: center; gap: 8px;">
            <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2.5" fill="none" style="stroke: var(--color-accent);"><path d="M9 18V5l12-2v13"></path><circle cx="6" cy="18" r="3"></circle><circle cx="18" cy="16" r="3"></circle></svg>
            Playlists
          </div>
          <div id="spotify-playlists-list" class="results-list" style="height: 380px; overflow-y: auto; background: rgba(255,255,255,0.01); border-radius: 6px; padding: 6px; border: 1px solid var(--border-light);">Loading Playlists...</div>
        </div>

        <!-- Right Column: Liked Songs & On Repeat stacked vertically -->
        <div class="spotify-dashboard-col">
          <!-- Liked Songs Section -->
          <div class="settings-group-card" style="max-width: 100%;">
            <div class="group-header" style="display: flex; align-items: center; gap: 8px;">
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2.5" fill="none" style="stroke: var(--color-accent);"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"></path></svg>
              Liked Songs
            </div>
            <div id="spotify-liked-tracks" class="results-list" style="height: 170px; overflow-y: auto; background: rgba(255,255,255,0.01); border-radius: 6px; padding: 6px; border: 1px solid var(--border-light);">Loading Liked Songs...</div>
          </div>

          <!-- On Repeat Section -->
          <div class="settings-group-card" style="max-width: 100%;">
            <div class="group-header" style="display: flex; align-items: center; justify-content: space-between;">
              <span style="display: flex; align-items: center; gap: 8px;">
                <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2.5" fill="none" style="stroke: var(--color-accent);"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>
                On Repeat
                <span id="on-repeat-source-badge" style="font-size: 0.62rem; padding: 1px 6px; border-radius: 4px; background: rgba(255,255,255,0.04); color: var(--color-text-dim); margin-left: 6px; font-weight: normal; text-transform: capitalize; display: none;"></span>
              </span>
              <button id="btn-header-import-onrepeat" class="ctrl-btn" style="font-size: 0.72rem; padding: 4px 10px; border-radius: 20px; border: 1px solid var(--border-light); background: rgba(255,255,255,0.03); color: var(--color-text-muted); font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 6px; transition: all 0.2s ease;">
                <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/></svg>
                Import
              </button>
            </div>
            <div id="spotify-on-repeat" class="results-list" style="height: 170px; overflow-y: auto; background: rgba(255,255,255,0.01); border-radius: 6px; padding: 6px; border: 1px solid var(--border-light);">Loading On Repeat...</div>
          </div>
        </div>
      </div>
    </div>
  `;

  // Attach event listeners
  const logoutBtn = document.getElementById('btn-spotify-logout');
  if (logoutBtn) {
    logoutBtn.addEventListener('click', logoutSpotify);
  }

  const headerImportBtn = document.getElementById('btn-header-import-onrepeat');
  if (headerImportBtn) {
    headerImportBtn.addEventListener('click', openSpotifyImportModal);
  }

  // 1. Fetch Liked Songs via Rust API Proxy
  invoke('spotify_fetch_liked_songs', { offset: 0, limit: 50, token: appState.spotifyToken })
    .then(tracks => {
      const likedContainer = document.getElementById('spotify-liked-tracks');
      if (!likedContainer) return;
      likedContainer.innerHTML = '';

      if (tracks.length > 0) {
        tracks.forEach((track, idx) => {
          const row = createTrackRowHTML(track, idx + 1);
          likedContainer.appendChild(row);
        });
      } else {
        likedContainer.innerHTML = '<div style="padding:15px; color:var(--color-text-dim); text-align:center;">No Liked Songs found.</div>';
      }
    })
    .catch(err => {
      const likedContainer = document.getElementById('spotify-liked-tracks');
      if (likedContainer) {
        likedContainer.innerHTML = `<div style="padding:15px; color:var(--color-danger); text-align:center;">Error: ${err}</div>`;
      }
    });

  // Pre-load from local storage cache if available
  const cachedOnRepeat = localStorage.getItem('spotify_on_repeat_imported');
  let hasCache = false;
  if (cachedOnRepeat) {
    try {
      const tracks = JSON.parse(cachedOnRepeat);
      if (tracks && tracks.length > 0) {
        renderOnRepeatTracks(tracks, true);
        hasCache = true;
        resolveSpotifyTrackArtworks(tracks);
      }
    } catch (e) {
      console.error("Failed to parse cached On Repeat tracks:", e);
    }
  }

  // 2. Fetch "On Repeat" Playlist Tracks
  invoke('spotify_fetch_playlist_tracks', { playlistId: '37i9dQZF1EpgP5h8orLeuN', token: appState.spotifyToken })
    .then(tracks => {
      if (tracks && tracks.length > 0) {
        renderOnRepeatTracks(tracks, false);
        // Also cache them if they successfully fetched
        localStorage.setItem('spotify_on_repeat_imported', JSON.stringify(tracks));
      } else if (!hasCache) {
        renderOnRepeatTracks([], false);
      }
    })
    .catch(err => {
      console.warn("Failed to fetch On Repeat from Spotify API, using local import if available. Error:", err);
      if (!hasCache) {
        const onRepeatContainer = document.getElementById('spotify-on-repeat');
        if (onRepeatContainer) {
          onRepeatContainer.innerHTML = `
            <div style="padding:20px; color:var(--color-text-dim); text-align:center;">
              <span style="color:var(--color-danger); display:block; margin-bottom:10px; font-size: 0.85rem;">Spotify API: ${err}</span>
              Unable to auto-fetch system playlist. Load your Exportify file instead:
              <br/>
              <button id="btn-trigger-import-onrepeat" class="primary-btn" style="margin-top:12px; padding:6px 14px; font-size:0.8rem; background:var(--color-accent); color:white; border:none; border-radius:4px; font-weight:600; cursor:pointer;">Import Exportify CSV / JSON</button>
            </div>
          `;
          const triggerBtn = document.getElementById('btn-trigger-import-onrepeat');
          if (triggerBtn) {
            triggerBtn.addEventListener('click', openSpotifyImportModal);
          }
        }
      }
    });

  // 3. Fetch Playlists via Rust API Proxy
  invoke('spotify_fetch_playlists', { token: appState.spotifyToken })
    .then(playlists => {
      const playlistContainer = document.getElementById('spotify-playlists-list');
      if (!playlistContainer) return;
      playlistContainer.innerHTML = '';

      if (playlists.length > 0) {
        playlists.forEach(playlist => {
          const div = document.createElement('div');
          div.className = 'track-row';
          div.style.cursor = 'pointer';
          div.innerHTML = `
            <img src="${playlist.thumbnail || '/assets/placeholder_art.png'}" class="track-row-art" />
            <div class="track-details-col">
              <span class="track-row-title">${playlist.name}</span>
              <span class="track-row-artist">${playlist.tracks_total} tracks</span>
            </div>
            <span class="source-tag spotify" style="margin-left:auto;">spotify</span>
          `;

          div.addEventListener('click', () => {
            loadSpotifyPlaylistTracks(playlist.id, playlist.name);
          });
          playlistContainer.appendChild(div);
        });
      } else {
        playlistContainer.innerHTML = '<div style="padding:15px; color:var(--color-text-dim); text-align:center;">No Playlists found.</div>';
      }
    })
    .catch(err => {
      const playlistContainer = document.getElementById('spotify-playlists-list');
      if (playlistContainer) {
        playlistContainer.innerHTML = `<div style="padding:15px; color:var(--color-danger); text-align:center;">Error: ${err}</div>`;
      }
    });
}

function loadSpotifyPlaylistTracks(playlistId, playlistName) {
  switchScreen('playlists');
  const detailContainer = document.getElementById('playlist-active-view');
  if (!detailContainer) return;
  detailContainer.innerHTML = `<div class="results-loading" style="text-align:center; padding: 40px; color:var(--color-text-dim);">Loading tracks from Spotify playlist "${playlistName}"...</div>`;

  console.log(`%c[Diagnostic] Clicking Spotify Playlist:\n- Name: ${playlistName}\n- ID: ${playlistId}`, "color: #1DB954; font-weight: bold;");

  // JS-side diagnostic fetch for Playlist Tracks
  fetch(`https://api.spotify.com/v1/playlists/${playlistId}`, {
    headers: { 'Authorization': `Bearer ${appState.spotifyToken}` }
  })
    .then(res => res.json())
    .then(data => {
      console.log("[Diagnostic] JavaScript-side Playlist Tracks fetch result:", data);
      console.log("[Diagnostic] Playlist keys:", Object.keys(data));
      console.log("[Diagnostic] data.items type/val:", typeof data.items, data.items);
      let items;
      if (data.tracks && Array.isArray(data.tracks.items)) {
        items = data.tracks.items;
      } else if (data.items && Array.isArray(data.items.items)) {
        items = data.items.items;
      } else if (Array.isArray(data.items)) {
        items = data.items;
      }

      if (items) {
        console.log("[Diagnostic] Tracks/items length found:", items.length);
        if (items.length > 0) {
          console.log("[Diagnostic] First track item:", items[0]);
        }
      } else {
        console.log("[Diagnostic] No tracks/items array found in data.tracks.items, data.items.items, or data.items!");
      }
    })
    .catch(err => console.error("[Diagnostic] JavaScript-side Playlist Tracks fetch failed:", err));

  invoke('spotify_fetch_playlist_tracks', { playlistId, token: appState.spotifyToken })
    .then(tracks => {
      // Construct a mock playlist object
      const spotifyPlaylist = {
        name: `Spotify: ${playlistName}`,
        tracks: tracks
      };

      renderPlaylistDetails(spotifyPlaylist, null);

      // Hide the "Add Track" and "Delete Playlist" buttons since this is a read-only Spotify playlist
      const deleteBtn = document.getElementById('btn-delete-playlist');
      if (deleteBtn) deleteBtn.style.display = 'none';
    })
    .catch(err => {
      detailContainer.innerHTML = `<div class="results-loading" style="color:var(--color-danger); text-align:center; padding: 40px;">Error loading tracks: ${err}</div>`;
    });
}

// Cryptographic helper functions deleted. Exchanged for secure native Rust-backed PKCE and hashing implementation.

// ==========================================================================
// 4. API Search Engines & Content Resolvers
// ==========================================================================
async function performSearch(query) {
  activeSearchQuery = query;

  // 1. YouTube Native Search via Rust (bypassing CORS)
  document.getElementById('youtube-loading').style.display = 'block';
  document.getElementById('youtube-results-list').innerHTML = '';

  invoke('search_youtube', { query: query })
    .then(res => {
      if (query !== activeSearchQuery) return;
      document.getElementById('youtube-loading').style.display = 'none';
      renderYouTubeResults(res);
    })
    .catch(err => {
      if (query !== activeSearchQuery) return;
      document.getElementById('youtube-loading').style.display = 'none';
      document.getElementById('youtube-results-list').innerHTML = `<div class="results-loading">Error: ${err}</div>`;
    });

  // 2. Spotify Direct API Search (Client Credentials / OAuth Token)
  const hasToken = appState.spotifyToken && appState.spotifyToken !== 'null' && appState.spotifyToken !== 'undefined';
  if (hasToken) {
    if (Date.now() > appState.spotifyTokenExpiry * 1000) {
      await refreshSpotifyAccessToken();
    }

    if (query !== activeSearchQuery) return;

    document.getElementById('spotify-loading').style.display = 'block';
    document.getElementById('spotify-results-list').innerHTML = '';

    fetch(`https://api.spotify.com/v1/search?q=${encodeURIComponent(query)}&type=track&limit=10`, {
      headers: { 'Authorization': `Bearer ${appState.spotifyToken}` }
    })
      .then(async res => {
        if (!res.ok) {
          let errMsg = `HTTP error! status: ${res.status}`;
          try {
            const errData = await res.json();
            if (errData.error && errData.error.message) {
              errMsg = `${errData.error.message} (status: ${res.status})`;
            }
          } catch (_) { }

          if (res.status === 401 && appState.spotifyRefreshToken) {
            console.warn('Spotify search returned 401. Attempting token refresh...');
            await refreshSpotifyAccessToken();
            // Retry fetch once with refreshed token
            const retryRes = await fetch(`https://api.spotify.com/v1/search?q=${encodeURIComponent(query)}&type=track&limit=10`, {
              headers: { 'Authorization': `Bearer ${appState.spotifyToken}` }
            });
            if (!retryRes.ok) {
              let retryErrMsg = `HTTP error! status: ${retryRes.status}`;
              try {
                const retryErrData = await retryRes.json();
                if (retryErrData.error && retryErrData.error.message) {
                  retryErrMsg = retryErrData.error.message;
                }
              } catch (_) { }
              throw new Error(retryErrMsg);
            }
            return retryRes.json();
          }
          throw new Error(errMsg);
        }
        return res.json();
      })
      .then(data => {
        if (query !== activeSearchQuery) return;
        document.getElementById('spotify-loading').style.display = 'none';
        if (data.tracks && data.tracks.items) {
          renderSpotifyResults(data.tracks.items);
        } else if (data.error) {
          throw new Error(data.error.message || 'Unknown Spotify API error');
        } else {
          document.getElementById('spotify-results-list').innerHTML = '<div class="search-initial">No tracks found.</div>';
        }
      })
      .catch(err => {
        if (query !== activeSearchQuery) return;
        document.getElementById('spotify-loading').style.display = 'none';
        document.getElementById('spotify-results-list').innerHTML = `<div class="results-loading">Error searching Spotify: ${err.message || err}</div>`;
      });
  } else {
    document.getElementById('spotify-results-list').innerHTML = `
      <div class="search-initial">
        <p>Spotify account not connected.</p>
        <button class="primary-btn wide-btn" style="margin-top: 10px;" onclick="window.switchScreen('spotify-connect')">Connect Spotify</button>
      </div>`;
  }
}

function clearSearchResults() {
  document.getElementById('spotify-results-list').innerHTML = '<div class="search-initial">Type a query to search high-fidelity Spotify tracks.</div>';
  document.getElementById('youtube-results-list').innerHTML = '<div class="search-initial">Type a query to search high-fidelity YouTube streams.</div>';
}

function renderYouTubeResults(items) {
  const container = document.getElementById('youtube-results-list');
  if (items.length === 0) {
    container.innerHTML = '<div class="search-initial">No results found.</div>';
    return;
  }

  items.forEach((video, idx) => {
    const track = {
      id: video.id,
      title: video.title,
      artist: video.channel,
      album: 'YouTube Stream',
      duration: video.duration,
      source: 'youtube',
      thumbnail: video.thumbnail,
    };

    const row = createTrackRowHTML(track, idx + 1);
    container.appendChild(row);
  });
}

function renderSpotifyResults(items) {
  const container = document.getElementById('spotify-results-list');
  if (items.length === 0) {
    container.innerHTML = '<div class="search-initial">No results found.</div>';
    return;
  }

  items.forEach((item, idx) => {
    // Format duration ms to String MM:SS
    const totalSec = Math.floor(item.duration_ms / 1000);
    const min = Math.floor(totalSec / 60);
    const sec = totalSec % 60;
    const durStr = `${min}:${sec < 10 ? '0' : ''}${sec}`;

    const track = {
      id: item.id,
      title: item.name,
      artist: item.artists.map(a => a.name).join(', '),
      album: item.album.name,
      duration: durStr,
      source: 'spotify',
      thumbnail: item.album.images[0]?.url || '/assets/placeholder_art.png',
    };

    const row = createTrackRowHTML(track, idx + 1);
    container.appendChild(row);
  });
}

function createTrackRowHTML(track, index) {
  const div = document.createElement('div');
  div.className = 'track-row';

  const artSrc = getArtworkUrl(track.thumbnail);
  div.innerHTML = `
    <div class="track-index-col">${index}</div>
    <img src="${artSrc}" class="track-row-art" />
    <div class="track-details-col">
      <span class="track-row-title">${track.title}</span>
      <span class="track-row-artist">${track.artist}</span>
    </div>
    <div class="track-row-album">${track.album}</div>
    <span class="source-tag ${track.source}">${track.source}</span>
    <div class="track-row-duration">${track.duration}</div>
    <button class="row-action-btn add-to-playlist" title="Add to Playlist">
      <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2" fill="none"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
    </button>
  `;

  // Single click plays track
  div.addEventListener('click', (e) => {
    if (e.target.closest('.row-action-btn')) return; // Ignore if adding to playlist
    playTrackImmediate(track);
  });

  div.querySelector('.add-to-playlist').addEventListener('click', (e) => {
    e.stopPropagation();
    promptAddTrackToPlaylist(track);
  });

  return div;
}

// ==========================================================================
// 5. Dual Playback Engine Router (Spotify SDK & YouTube API)
// ==========================================================================
window.onYouTubeIframeAPIReady = () => {
  ytPlayer = new YT.Player('yt-player-container', {
    height: '100%',
    width: '100%',
    videoId: '',
    playerVars: {
      'playsinline': 1,
      'controls': 0,
      'disablekb': 1,
      'enablejsapi': 1,
      'origin': window.location.origin
    },
    events: {
      'onStateChange': onYouTubePlayerStateChange,
      'onReady': onYouTubePlayerReady,
    }
  });
};

function onYouTubePlayerReady(event) {
  // Sync initial volume
  ytPlayer.setVolume(appState.volume);
}

function onYouTubePlayerStateChange(event) {
  if (event.data === YT.PlayerState.PLAYING) {
    startTimelineProgressTracker();
    setPlayState(true);
  } else if (event.data === YT.PlayerState.PAUSED || event.data === YT.PlayerState.ENDED) {
    clearInterval(timelineInterval);
    setPlayState(false);

    if (event.data === YT.PlayerState.ENDED) {
      handleTrackFinished();
    }
  }
}

// Dynamic script loaders
const ytScript = document.createElement('script');
ytScript.src = "https://www.youtube.com/iframe_api";
document.head.appendChild(ytScript);

async function initNativeSpotifyEvents() {
  try {
    const { listen } = window.__TAURI__.event;

    await listen('spotify-playback-event', (event) => {
      const payload = event.payload;
      console.log('Received native Spotify event:', payload);

      if (payload.type === 'StateChanged') {
        const state = payload.data;

        // Sync play state
        setPlayState(state.is_playing);

        // Sync timeline position
        const cur = state.position_ms / 1000;
        const durationMs = appState.currentTrack ? getTrackDurationMs(appState.currentTrack) : 0;
        if (durationMs > 0) {
          const pct = (state.position_ms / durationMs) * 100;
          document.getElementById('player-seek-slider').value = pct;
          document.getElementById('player-seek-fill').style.width = pct + '%';
          document.getElementById('player-time-current').textContent = formatSeconds(cur);

          if (appState.currentLyrics && appState.currentLyrics.synced) {
            syncActiveLyricLine(state.position_ms);
          }
        }
      } else if (payload.type === 'NextTrack') {
        playNext();
      } else if (payload.type === 'PrevTrack') {
        playPrev();
      } else if (payload.type === 'EndOfTrack') {
        handleTrackFinished();
      } else if (payload.type === 'TimeToPreloadNextTrack') {
        preloadNextTrack();
      }
    });

    await listen('queue-updated', (event) => {
      const queue = event.payload;
      console.log('Received native queue updated event:', queue);
      appState.queue = queue.items;
      appState.queueIndex = queue.current_index;
      appState.isShuffle = queue.is_shuffle;
      appState.repeatMode = queue.repeat_mode;

      document.getElementById('ctrl-shuffle').classList.toggle('active', appState.isShuffle);

      const repeatBtn = document.getElementById('ctrl-repeat');
      if (appState.repeatMode === 'track') {
        repeatBtn.classList.add('active');
        repeatBtn.setAttribute('title', 'Repeat: Track');
      } else if (appState.repeatMode === 'queue') {
        repeatBtn.classList.add('active');
        repeatBtn.setAttribute('title', 'Repeat: Queue');
      } else {
        repeatBtn.classList.remove('active');
        repeatBtn.setAttribute('title', 'Repeat: Off');
      }

      renderQueueSidebar();
    });
  } catch (err) {
    console.error('Failed to register native Spotify events:', err);
  }
}

function preloadNextTrack() {
  if (!appState.spotifySettings.gapless) return;
  if (appState.queue.length === 0) return;

  let nextIndex = appState.queueIndex + 1;
  if (nextIndex >= appState.queue.length) {
    if (appState.repeatMode === 'context') {
      nextIndex = 0;
    } else {
      return; // No next track to preload
    }
  }

  if (nextTrack && nextTrack.source === 'spotify') {
    console.log("Preloading next Spotify track natively:", nextTrack.title);
    invoke('spotify_preload', { trackId: nextTrack.id }).catch(console.error);
  }
}

function getTauriAssetUrl(filePath) {
  if (!filePath) return 'assets/default_art.png';
  let cleanPath = filePath;
  if (cleanPath.startsWith('file:///')) {
    cleanPath = cleanPath.slice(7); // strips file:// leaving /home/... or /C:/...
  } else if (cleanPath.startsWith('file://')) {
    cleanPath = cleanPath.slice(7);
  }
  // Strip leading slash on Windows drive letter paths (e.g. /C:/ -> C:/)
  if (cleanPath.startsWith('/') && cleanPath.charAt(2) === ':') {
    cleanPath = cleanPath.slice(1);
  }

  if (window.__TAURI__) {
    if (window.__TAURI__.core && typeof window.__TAURI__.core.convertFileSrc === 'function') {
      return window.__TAURI__.core.convertFileSrc(cleanPath);
    }
    if (window.__TAURI__.primitives && typeof window.__TAURI__.primitives.convertFileSrc === 'function') {
      return window.__TAURI__.primitives.convertFileSrc(cleanPath);
    }
    if (typeof window.__TAURI__.convertFileSrc === 'function') {
      return window.__TAURI__.convertFileSrc(cleanPath);
    }
  }
  const cleanPathUrl = cleanPath.startsWith('/') ? cleanPath.slice(1) : cleanPath;
  return `https://asset.tauri.localhost/${cleanPathUrl}`;
}

function getArtworkUrl(thumbnail) {
  if (!thumbnail) return 'assets/default_art.png';
  if (thumbnail.startsWith('/') || thumbnail.startsWith('C:\\') || thumbnail.startsWith('file://') || thumbnail.includes('com.zech.crumusix') || thumbnail.includes('crumusix_cache')) {
    return getTauriAssetUrl(thumbnail);
  }
  return thumbnail;
}

function getTrackDurationMs(track) {
  if (!track || !track.duration) return 0;
  if (/^\d+$/.test(track.duration.toString().trim())) {
    return parseInt(track.duration.toString().trim()) * 1000;
  }
  const parts = track.duration.split(':');
  if (parts.length === 2) {
    const min = parseInt(parts[0]);
    const sec = parseInt(parts[1]);
    return (min * 60 + sec) * 1000;
  } else if (parts.length === 3) {
    const hrs = parseInt(parts[0]);
    const min = parseInt(parts[1]);
    const sec = parseInt(parts[2]);
    return (hrs * 3600 + min * 60 + sec) * 1000;
  }
  return 0;
}

// Plays any track, managing transitions between active play state channels
async function playTrackImmediate(track) {
  appState.currentTrack = track;
  appState.lyricOffsetMs = 0;
  appState.prefetchedNextLyrics = false;

  const offsetLabel = document.getElementById('lyric-sync-offset-label');
  if (offsetLabel) {
    offsetLabel.textContent = '0.0s';
  }

  // Transition dynamic ambient backdrop colors
  updateLyricsBackdropColors(track.thumbnail, track.id, track.artist);

  updatePlayerUI(track);
  addToHistory(track);

  // Set active source indicators
  const indicator = document.getElementById('player-source-indicator');
  indicator.textContent = track.source === 'spotify' ? 'SP' : (track.source === 'youtube' ? 'YT' : 'LC');
  indicator.className = `source-icon ${track.source}`;

  if (track.source === 'local') {
    invoke('spotify_stop').catch(console.error);
    if (ytPlayer) ytPlayer.pauseVideo();
    document.getElementById('compact-video-view').classList.add('video-corner-hidden');

    appState.local_position_ms = 0;
    try {
      await invoke('local_play', { trackId: track.id });
      setPlayState(true);
      startTimelineProgressTracker();
    } catch (err) {
      console.error('Failed to play local track:', err);
    }
  } else if (track.source === 'youtube') {
    // 1. YouTube Channel
    invoke('spotify_stop').catch(console.error);

    // Load track in YT Player
    if (ytPlayer && ytPlayer.loadVideoById) {
      ytPlayer.loadVideoById(track.id);

      // Control compact corner view overlay visibility using classes, NEVER alter the iframe DOM tree!
      const showVideo = document.getElementById('setting-show-video').checked;
      const videoCorner = document.getElementById('compact-video-view');

      if (showVideo) {
        videoCorner.classList.remove('video-corner-hidden');
      } else {
        videoCorner.classList.add('video-corner-hidden');
      }

      ytPlayer.playVideo();
    } else {
      console.warn("YouTube player not ready yet, retrying in 250ms...");
      setTimeout(() => playTrackImmediate(track), 250);
    }
  } else if (track.source === 'spotify') {
    // 2. Spotify Channel (Direct Streaming)
    if (ytPlayer) ytPlayer.pauseVideo();
    document.getElementById('compact-video-view').classList.add('video-corner-hidden');

    if (!appState.spotifyToken && !appState.spotifyDeviceId) {
      const connect = await confirm("Spotify account is not connected.\n\nWould you like to go to the Spotify Auth screen now, or search and play this track on YouTube instead?");
      if (connect) {
        switchScreen('spotify-connect');
      } else {
        await fallbackSpotifyToYouTube(track);
      }
      return;
    }

    try {
      let finalTrack = track;
      // Check smart sled cache first
      try {
        const cached = await invoke('cache_get_track', { id: track.id });
        if (cached) {
          console.log("Smart cache hit for Spotify track:", cached);

          // Clear corrupted artwork URLs from previous versions to force fresh fetch/cache
          if (cached.artwork_url && (cached.artwork_url.includes('file:') || cached.artwork_url.includes('asset:') || cached.artwork_url.includes('crumusix_cache') || cached.artwork_url.startsWith('data:'))) {
            console.log("Clearing corrupted artwork URL from smart cache for track:", cached.id);
            cached.artwork_url = "";
            await invoke('cache_set_track', { track: cached }).catch(console.error);
          }

          finalTrack = {
            id: cached.id,
            title: cached.title,
            artist: cached.artist,
            album: cached.album,
            duration: formatSeconds(cached.duration_ms / 1000),
            source: 'spotify',
            thumbnail: cached.artwork_url
          };
        } else {
          // Set cache on initial fetch/play
          const durationMs = getTrackDurationMs(track);
          await invoke('cache_set_track', {
            track: {
              id: track.id,
              title: track.title,
              artist: track.artist,
              album: track.album || "",
              duration_ms: durationMs,
              artwork_url: track.thumbnail || ""
            }
          });
          console.log("Cached Spotify track metadata for future plays");
        }
      } catch (cacheErr) {
        console.error("Cache operation failed, playing with original metadata:", cacheErr);
      }

      // Check / Cache local downsampled artwork WebP
      let artworkUrl = finalTrack.thumbnail;
      if (artworkUrl) {
        try {
          const cachedArtworkPath = await invoke('cache_get_artwork', {
            url: artworkUrl,
            identifier: finalTrack.id
          });
          if (cachedArtworkPath) {
            console.log("Using cached local artwork path (base64 data URL)");
            finalTrack.thumbnail = cachedArtworkPath;

            // Update UI with the cached artwork immediately
            const playerArt = document.getElementById('player-album-art');
            if (playerArt) playerArt.src = cachedArtworkPath;
          }
        } catch (artErr) {
          console.error("Artwork caching failed, fallback to remote URL:", artErr);
        }
      }

      appState.currentTrack = finalTrack;
      updatePlayerUI(finalTrack);

      const durationMs = getTrackDurationMs(finalTrack);
      await invoke('spotify_play', {
        trackId: finalTrack.id,
        title: finalTrack.title,
        artist: finalTrack.artist,
        album: finalTrack.album || "",
        durationMs: durationMs,
        thumbnail: finalTrack.thumbnail || ""
      });
    } catch (err) {
      console.error('Failed to play native Spotify track:', err);
      await fallbackSpotifyToYouTube(track);
    }
  }
}

async function fallbackSpotifyToYouTube(spotifyTrack) {
  const query = `${spotifyTrack.title} ${spotifyTrack.artist}`;
  console.log(`Falling back Spotify track to YouTube search for: ${query}`);

  const titleEl = document.getElementById('player-track-title');
  const artistEl = document.getElementById('player-track-artist');

  titleEl.textContent = "Searching YouTube...";
  artistEl.textContent = `Finding fallback for "${spotifyTrack.title}"`;

  try {
    const results = await invoke('search_youtube', { query: query });
    if (results && results.length > 0) {
      const bestMatch = results[0];
      const ytTrack = {
        id: bestMatch.id,
        title: spotifyTrack.title, // Keep original Spotify metadata for clean UI
        artist: spotifyTrack.artist,
        album: spotifyTrack.album || 'YouTube fallback',
        duration: bestMatch.duration,
        source: 'youtube',
        thumbnail: spotifyTrack.thumbnail || bestMatch.thumbnail,
      };

      console.log(`Fallback found: ${bestMatch.title} (${bestMatch.id})`);
      playTrackImmediate(ytTrack);
    } else {
      await alert(`Could not find a YouTube fallback for "${spotifyTrack.title}"`);
      titleEl.textContent = "Not Playing";
      artistEl.textContent = "Select a song to start";
    }
  } catch (err) {
    console.error("YouTube fallback search failed:", err);
    await alert(`YouTube fallback failed: ${err}`);
    titleEl.textContent = "Not Playing";
    artistEl.textContent = "Select a song to start";
  }
}

function extractDominantColor(imgElement) {
  return new Promise((resolve) => {
    if (!imgElement || !imgElement.complete || imgElement.naturalWidth === 0) {
      resolve(null);
      return;
    }

    try {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      canvas.width = 30;
      canvas.height = 30;

      // Handle CORS safely
      imgElement.crossOrigin = "anonymous";

      ctx.drawImage(imgElement, 0, 0, 30, 30);
      const imgData = ctx.getImageData(0, 0, 30, 30).data;

      let rSum = 0, gSum = 0, bSum = 0, count = 0;

      for (let i = 0; i < imgData.length; i += 4) {
        const r = imgData[i];
        const g = imgData[i + 1];
        const b = imgData[i + 2];
        const a = imgData[i + 3];

        // Skip excessively dark/bright pixels
        if (a > 200 && (r + g + b > 60) && (r + g + b < 680)) {
          rSum += r;
          gSum += g;
          bSum += b;
          count++;
        }
      }

      if (count === 0) {
        resolve({ r: 168, g: 85, b: 247 }); // Default Brand Purple
        return;
      }

      resolve({
        r: Math.round(rSum / count),
        g: Math.round(gSum / count),
        b: Math.round(bSum / count)
      });
    } catch (e) {
      console.warn("Failed to extract color palette:", e);
      resolve({ r: 168, g: 85, b: 247 });
    }
  });
}

function rgbToHsl(r, g, b) {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h, s, l = (max + min) / 2;

  if (max === min) {
    h = s = 0;
  } else {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = (g - b) / d + (g < b ? 6 : 0); break;
      case g: h = (b - r) / d + 2; break;
      case b: h = (r - g) / d + 4; break;
    }
    h /= 6;
  }

  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100)
  };
}

async function updateDynamicTheme(imgElement) {
  // Lock theme accent to a single solid color
  return;

  const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
  const h = hsl.h;
  const s = Math.max(hsl.s, 65); // High vibrancy
  const l = Math.min(Math.max(hsl.l, 48), 65); // Controlled contrast

  const root = document.documentElement;
  root.style.setProperty('--color-accent', `hsl(${h}, ${s}%, ${l}%)`);
  root.style.setProperty('--accent-secondary', `hsl(${h}, ${s}%, ${Math.max(l - 15, 30)}%)`);
  root.style.setProperty('--ambient-glow', `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.18)`);
  root.style.setProperty('--border-glow', `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.25)`);
}

function updatePlayerUI(track) {
  document.getElementById('player-track-title').textContent = track.title;
  document.getElementById('player-track-artist').textContent = track.artist;
  let artSrc = 'assets/default_art.png';
  if (track.thumbnail) {
    if (track.thumbnail.startsWith('/') || track.thumbnail.startsWith('C:\\') || track.thumbnail.includes('com.zech.crumusix') || track.thumbnail.includes('crumusix_cache')) {
      artSrc = getTauriAssetUrl(track.thumbnail);
    } else {
      artSrc = track.thumbnail;
    }
  }

  const playerArt = document.getElementById('player-album-art');
  playerArt.src = artSrc;

  // Extract theme colors when artwork loads
  playerArt.onload = () => {
    updateDynamicTheme(playerArt);
  };

  // Also load immediately in case it is already cached/complete
  if (playerArt.complete) {
    updateDynamicTheme(playerArt);
  }

  document.getElementById('sidebar-artwork').src = artSrc;
  document.getElementById('player-time-total').textContent = track.duration;

  // Sidebar artwork spinning status
  document.getElementById('sidebar-artwork').classList.add('playing');
}

function setPlayState(playing) {
  appState.isPlaying = playing;

  const playBtn = document.getElementById('play-icon');
  const pauseBtn = document.getElementById('pause-icon');
  const sidebarArtwork = document.getElementById('sidebar-artwork');

  if (playing) {
    playBtn.style.display = 'none';
    pauseBtn.style.display = 'block';
    if (sidebarArtwork) sidebarArtwork.classList.add('playing');
  } else {
    playBtn.style.display = 'block';
    pauseBtn.style.display = 'none';
    if (sidebarArtwork) sidebarArtwork.classList.remove('playing');
  }
}

function togglePlayPause() {
  if (!appState.currentTrack) return;

  if (appState.isPlaying) {
    if (appState.currentTrack.source === 'youtube' && ytPlayer) {
      ytPlayer.pauseVideo();
    } else if (appState.currentTrack.source === 'spotify') {
      invoke('spotify_pause').catch(console.error);
    } else if (appState.currentTrack.source === 'local') {
      invoke('local_pause').catch(console.error);
      setPlayState(false);
    }
  } else {
    if (appState.currentTrack.source === 'youtube' && ytPlayer) {
      ytPlayer.playVideo();
    } else if (appState.currentTrack.source === 'spotify') {
      invoke('spotify_resume').catch(console.error);
    } else if (appState.currentTrack.source === 'local') {
      invoke('local_play', { trackId: appState.currentTrack.id }).catch(console.error);
      setPlayState(true);
      startTimelineProgressTracker();
    }
  }
}

function seekPlayback(percent) {
  if (!appState.currentTrack) return;

  if (appState.currentTrack.source === 'youtube' && ytPlayer) {
    const dur = ytPlayer.getDuration();
    const target = (percent / 100) * dur;
    ytPlayer.seekTo(target, true);
  } else if (appState.currentTrack.source === 'spotify') {
    const durationMs = getTrackDurationMs(appState.currentTrack);
    if (durationMs > 0) {
      const targetMs = Math.floor((percent / 100) * durationMs);
      invoke('spotify_seek', { positionMs: targetMs }).catch(console.error);
    }
  } else if (appState.currentTrack.source === 'local') {
    const durationMs = getTrackDurationMs(appState.currentTrack);
    if (durationMs > 0) {
      const targetMs = Math.floor((percent / 100) * durationMs);
      appState.local_position_ms = targetMs;
      invoke('local_seek', { positionMs: targetMs }).catch(console.error);
    }
  }
}

function setVolume(val) {
  if (ytPlayer && ytPlayer.setVolume) {
    ytPlayer.setVolume(val);
  }
  if (appState.currentTrack) {
    if (appState.currentTrack.source === 'spotify') {
      invoke('spotify_volume', { volume: val / 100 }).catch(console.error);
      if (appState.spotifyToken) {
        fetch(`https://api.spotify.com/v1/me/player/volume?volume_percent=${val}`, {
          method: 'PUT',
          headers: { 'Authorization': `Bearer ${appState.spotifyToken}` }
        }).catch(console.error);
      }
    } else if (appState.currentTrack.source === 'local') {
      invoke('local_volume', { volume: val / 100 }).catch(console.error);
    }
  }
}

function startTimelineProgressTracker() {
  clearInterval(timelineInterval);

  timelineInterval = setInterval(() => {
    if (!appState.isPlaying) return;

    if (appState.currentTrack.source === 'youtube' && ytPlayer && ytPlayer.getCurrentTime) {
      const cur = ytPlayer.getCurrentTime();
      const dur = ytPlayer.getDuration();
      if (dur > 0) {
        const pct = (cur / dur) * 100;
        document.getElementById('player-seek-slider').value = pct;
        document.getElementById('player-seek-fill').style.width = pct + '%';
        document.getElementById('player-time-current').textContent = formatSeconds(cur);

        // Dynamic prefetch trigger
        if (pct > 85 && !appState.prefetchedNextLyrics) {
          appState.prefetchedNextLyrics = true;
          prefetchUpcomingTrackLyrics();
        }

        if (appState.currentLyrics && appState.currentLyrics.synced) {
          syncActiveLyricLine(cur * 1000);
        }
      }
    } else if (appState.currentTrack.source === 'spotify') {
      // Local timeline estimation
      const slider = document.getElementById('player-seek-slider');
      const durationMs = getTrackDurationMs(appState.currentTrack);
      if (durationMs > 0) {
        let curMs = parseFloat(slider.value) * durationMs / 100;
        curMs += 500; // Increment 500ms
        const curSec = curMs / 1000;
        const pct = (curMs / durationMs) * 100;
        slider.value = pct;
        document.getElementById('player-seek-fill').style.width = pct + '%';
        document.getElementById('player-time-current').textContent = formatSeconds(curSec);

        // Dynamic prefetch trigger
        if (pct > 85 && !appState.prefetchedNextLyrics) {
          appState.prefetchedNextLyrics = true;
          prefetchUpcomingTrackLyrics();
        }

        if (appState.currentLyrics && appState.currentLyrics.synced) {
          syncActiveLyricLine(curMs);
        }
      }
    } else if (appState.currentTrack.source === 'local') {
      const slider = document.getElementById('player-seek-slider');
      const durationMs = getTrackDurationMs(appState.currentTrack);
      if (durationMs > 0) {
        if (appState.local_position_ms === undefined) {
          appState.local_position_ms = 0;
        }
        appState.local_position_ms += 500;
        const curSec = appState.local_position_ms / 1000;
        const pct = (appState.local_position_ms / durationMs) * 100;
        slider.value = pct;
        document.getElementById('player-seek-fill').style.width = pct + '%';
        document.getElementById('player-time-current').textContent = formatSeconds(curSec);

        if (pct > 85 && !appState.prefetchedNextLyrics) {
          appState.prefetchedNextLyrics = true;
          prefetchUpcomingTrackLyrics();
        }

        if (appState.currentLyrics && appState.currentLyrics.synced) {
          syncActiveLyricLine(appState.local_position_ms);
        }

        if (appState.local_position_ms >= durationMs) {
          clearInterval(timelineInterval);
          handleTrackFinished();
        }
      }
    }
  }, 500);
}

function formatSeconds(secs) {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s < 10 ? '0' : ''}${s}`;
}

async function playNext() {
  if (appState.queue.length === 0) return;

  let nextIdx = appState.queueIndex + 1;
  if (nextIdx >= appState.queue.length) {
    if (appState.repeatMode === 'queue') {
      nextIdx = 0;
    } else {
      console.log("End of queue reached.");
      return;
    }
  }

  try {
    await invoke('queue_set_current_index', { index: nextIdx });
    playTrackImmediate(appState.queue[nextIdx]);
  } catch (err) {
    console.error("Failed to play next track:", err);
  }
}

async function playPrev() {
  if (appState.queue.length === 0) return;

  let prevIdx = appState.queueIndex - 1;
  if (prevIdx < 0) {
    if (appState.repeatMode === 'queue') {
      prevIdx = appState.queue.length - 1;
    } else {
      prevIdx = 0;
    }
  }

  try {
    await invoke('queue_set_current_index', { index: prevIdx });
    playTrackImmediate(appState.queue[prevIdx]);
  } catch (err) {
    console.error("Failed to play previous track:", err);
  }
}

function handleTrackFinished() {
  if (appState.repeatMode === 'track') {
    playTrackImmediate(appState.currentTrack);
  } else {
    playNext();
  }
}

// ==========================================================================
// 6. Local Custom Playlist Persistence (Tauri Bridge JSON Database)
// ==========================================================================
function loadPlaylists() {
  invoke('get_playlists')
    .then(playlists => {
      appState.playlists = playlists;
      renderPlaylistsNav();
    })
    .catch(err => console.error('Failed to load playlists:', err));
}

function renderPlaylistsNav() {
  const container = document.getElementById('playlists-nav-list');
  container.innerHTML = '';

  if (appState.playlists.length === 0) {
    container.innerHTML = '<div style="padding: 10px; color:var(--color-text-dim); text-align:center;">No playlists yet.</div>';
    return;
  }

  appState.playlists.forEach(playlist => {
    const item = document.createElement('button');
    item.className = 'playlist-nav-item';
    if (appState.activePlaylist && appState.activePlaylist.name === playlist.name) {
      item.classList.add('active');
    }

    item.innerHTML = `
      <span>${playlist.name}</span>
      <span style="font-size:0.8rem; opacity:0.6;">${playlist.tracks.length}</span>
    `;

    item.addEventListener('click', () => {
      appState.activePlaylist = playlist;
      renderPlaylistsNav();
      renderPlaylistDetails(playlist);
    });

    container.appendChild(item);
  });
}

function renderPlaylistDetails(playlist, rawData = null) {
  const detailContainer = document.getElementById('playlist-active-view');

  detailContainer.innerHTML = `
    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom: 20px;">
      <div>
        <h2 style="font-family:var(--font-display); font-weight:800; margin: 0 0 4px 0;">${playlist.name}</h2>
        <p style="color:var(--color-text-muted); font-size:0.9rem; margin: 0;">${playlist.tracks.length} tracks inside local workspace</p>
      </div>
      <div style="display:flex; gap: 8px;">
        <button class="primary-btn" id="btn-play-playlist">Play All</button>
        <button class="danger-outline-btn" id="btn-delete-playlist">Delete</button>
      </div>
    </div>
    <div class="results-list" id="playlist-tracks-list"></div>
  `;

  const tracksList = document.getElementById('playlist-tracks-list');
  if (playlist.tracks.length === 0) {
    if (playlist.name.startsWith("Spotify:")) {
      let debugInfo = '';
      if (rawData) {
        debugInfo = `
          <pre style="text-align: left; background: rgba(0,0,0,0.4); padding: 12px; border-radius: 8px; font-size: 0.8rem; overflow-x: auto; max-width: 500px; margin: 15px auto 0; border: 1px solid rgba(255,255,255,0.05); color: #bbb; font-family: monospace; white-space: pre-wrap; word-break: break-all;">Diagnostic Logs:\n${JSON.stringify({
          keys: Object.keys(rawData),
          total: rawData.total,
          limit: rawData.limit,
          offset: rawData.offset,
          items_length: rawData.items ? rawData.items.length : 'undefined',
          first_item_keys: (rawData.items && rawData.items[0]) ? Object.keys(rawData.items[0]) : 'none',
          first_item_track_keys: (rawData.items && rawData.items[0] && rawData.items[0].track) ? Object.keys(rawData.items[0].track) : 'none'
        }, null, 2)}</pre>`;
      }
      tracksList.innerHTML = `
        <div class="search-initial" style="padding: 35px 20px; text-align:center;">
          <svg viewBox="0 0 24 24" width="48" height="48" stroke="var(--color-text-dim)" stroke-width="1.5" fill="none" style="margin-bottom:12px;"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
          <p style="margin-bottom: 8px; color: var(--color-text-muted); font-weight: 500;">No tracks returned or permissions missing</p>
          <p style="font-size:0.85rem; max-width: 450px; margin: 0 auto 16px; color: var(--color-text-dim); line-height: 1.5;">
            If this playlist has tracks on your Spotify account, your active login session lacks the required scopes. Please disconnect and reconnect your account to grant playlist permissions.
          </p>
          <button class="primary-btn" onclick="window.switchScreen('spotify-connect')">Go to Spotify Auth</button>
          ${debugInfo}
        </div>`;
    } else {
      tracksList.innerHTML = '<div class="search-initial">This playlist has no songs yet. Search above to add some!</div>';
    }
  } else {
    const isReadOnly = playlist.name.startsWith("Spotify:");
    playlist.tracks.forEach((track, idx) => {
      const row = document.createElement('div');
      row.className = 'track-row';
      const artSrc = getArtworkUrl(track.thumbnail);
      row.innerHTML = `
        <div class="track-index-col">${idx + 1}</div>
        <img src="${artSrc}" class="track-row-art" />
        <div class="track-details-col">
          <span class="track-row-title">${track.title}</span>
          <span class="track-row-artist">${track.artist}</span>
        </div>
        <div class="track-row-album">${track.album}</div>
        <span class="source-tag ${track.source}">${track.source}</span>
        <div class="track-row-duration">${track.duration}</div>
        ${isReadOnly ? '' : `
        <button class="row-action-btn remove-from-playlist" title="Remove track">
          <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2" fill="none"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
        `}
      `;

      row.addEventListener('click', (e) => {
        if (e.target.closest('.row-action-btn')) return;

        playTrackFromPlaylistNative(playlist.tracks, idx);
      });

      if (!isReadOnly) {
        row.querySelector('.remove-from-playlist').addEventListener('click', (e) => {
          e.stopPropagation();
          removeTrackFromPlaylist(playlist.name, track.id);
        });
      }

      tracksList.appendChild(row);
    });
  }

  // Play All setup
  document.getElementById('btn-play-playlist').addEventListener('click', () => {
    if (playlist.tracks.length === 0) return;
    playPlaylistNative(playlist.tracks);
  });

  // Delete playlist setup
  document.getElementById('btn-delete-playlist').addEventListener('click', async () => {
    if (await confirm(`Are you sure you want to delete the playlist "${playlist.name}"?`)) {
      invoke('delete_playlist', { name: playlist.name })
        .then(() => {
          appState.activePlaylist = null;
          loadPlaylists();
          detailContainer.innerHTML = `
            <div class="playlist-empty-view">
              <svg viewBox="0 0 24 24" width="48" height="48" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"></path><circle cx="6" cy="18" r="3"></circle><circle cx="18" cy="16" r="3"></circle></svg>
              <h3>Select or Create a Playlist</h3>
              <p>Organize both Spotify and YouTube tracks in a single local workspace.</p>
            </div>
          `;
        })
        .catch(err => console.error('Failed to delete playlist:', err));
    }
  });
}

async function createNewPlaylist() {
  const name = await prompt("Enter new playlist name:");
  if (!name) return;

  const trimName = name.trim();
  if (appState.playlists.some(p => p.name.toLowerCase() === trimName.toLowerCase())) {
    await alert("A playlist with that name already exists!");
    return;
  }

  const newPlaylist = { name: trimName, tracks: [] };
  invoke('save_playlist', { playlist: newPlaylist })
    .then(() => {
      loadPlaylists();
    })
    .catch(err => console.error('Failed to create playlist:', err));
}

async function promptAddTrackToPlaylist(track) {
  if (appState.playlists.length === 0) {
    const create = await confirm("You don't have any playlists yet. Create one now?");
    if (create) {
      await createNewPlaylist();
    }
    return;
  }

  // Custom simple dropdown/selection list dialog using simple prompt
  const options = appState.playlists.map((p, i) => `${i + 1}. ${p.name}`).join('\n');
  const choice = await prompt(`Select a playlist number to add this track to:\n\n${options}`);

  if (choice) {
    const idx = parseInt(choice.trim()) - 1;
    if (idx >= 0 && idx < appState.playlists.length) {
      const playlist = appState.playlists[idx];

      // Prevent duplicates in playlist
      if (playlist.tracks.some(t => t.id === track.id)) {
        await alert("This track is already inside the playlist!");
        return;
      }

      playlist.tracks.push(track);
      invoke('save_playlist', { playlist: playlist })
        .then(async () => {
          loadPlaylists();
          if (appState.activePlaylist && appState.activePlaylist.name === playlist.name) {
            appState.activePlaylist = playlist;
            renderPlaylistDetails(playlist);
          }
          await alert(`Successfully added to "${playlist.name}"!`);
        })
        .catch(err => console.error('Failed to append track:', err));
    }
  }
}

function removeTrackFromPlaylist(playlistName, trackId) {
  const playlist = appState.playlists.find(p => p.name === playlistName);
  if (!playlist) return;

  playlist.tracks = playlist.tracks.filter(t => t.id !== trackId);
  invoke('save_playlist', { playlist: playlist })
    .then(() => {
      loadPlaylists();
      appState.activePlaylist = playlist;
      renderPlaylistDetails(playlist);
    })
    .catch(err => console.error('Failed to remove track:', err));
}

// ==========================================================================
// 7. Recent Tracks History
// ==========================================================================
function addToHistory(track) {
  let list = appState.recentTracks.filter(t => t.id !== track.id);
  list.unshift(track); // prepend

  if (list.length > 10) {
    list.pop(); // Cap at 10 items
  }

  appState.recentTracks = list;
  saveConfig();
  renderRecentTracks();
}

function renderRecentTracks() {
  const container = document.getElementById('recent-tracks-container');
  if (!container) return;

  container.innerHTML = '';
  if (appState.recentTracks.length === 0) {
    container.innerHTML = '<div class="empty-state">No tracks played yet. Search above to start!</div>';
    return;
  }

  appState.recentTracks.forEach((track, idx) => {
    const card = document.createElement('div');
    card.className = 'track-row';
    card.style.margin = '4px 0';

    const artSrc = getArtworkUrl(track.thumbnail);
    card.innerHTML = `
      <div class="track-index-col">${idx + 1}</div>
      <img src="${artSrc}" class="track-row-art" />
      <div class="track-details-col">
        <span class="track-row-title">${track.title}</span>
        <span class="track-row-artist">${track.artist}</span>
      </div>
      <span class="source-tag ${track.source}">${track.source}</span>
      <div class="track-row-duration">${track.duration}</div>
    `;

    card.addEventListener('click', () => {
      playTrackImmediate(track);
    });

    container.appendChild(card);
  });
}

// Cinematic HTML5 Canvas Audio Particle Visualizer removed for premium flat-slate graphics optimization.

// ==========================================================================
// 9. Automated System Diagnostics Suite
// ==========================================================================
async function runSystemDiagnostics() {
  const logEl = document.getElementById('diagnostics-log');
  logEl.style.display = 'block';
  logEl.innerHTML = '';

  function log(msg, type = 'info') {
    let color = 'var(--color-text-main)';
    if (type === 'success') color = 'var(--color-spotify)';
    if (type === 'error') color = '#ef4444';
    if (type === 'warning') color = '#f59e0b';
    logEl.innerHTML += `<span style="color: ${color}">[${new Date().toLocaleTimeString()}] ${msg}</span>\n`;
    logEl.scrollTop = logEl.scrollHeight;
  }

  log('Starting CrumusiX Full System Diagnostics...', 'warning');

  // Test 1: Tauri IPC connection
  log('Test 1: Testing Tauri native bridge connection...');
  try {
    const playlists = await invoke('get_playlists');
    log(`SUCCESS: Tauri connection active. Found ${playlists.length} playlists.`, 'success');
  } catch (err) {
    log(`ERROR: Tauri connection failed: ${err}`, 'error');
    return;
  }

  // Test 2: YouTube scraper search
  log('Test 2: Testing YouTube search proxy (scraping ytInitialData)...');
  try {
    const results = await invoke('search_youtube', { query: 'linkin park' });
    if (results.length > 0) {
      log(`SUCCESS: YouTube scraper active. Found ${results.length} tracks. Top result: "${results[0].title}" by "${results[0].channel}"`, 'success');
    } else {
      log('WARNING: YouTube scraper returned 0 results. Key signature might have changed or network block.', 'error');
    }
  } catch (err) {
    log(`ERROR: YouTube scraper failed: ${err}`, 'error');
  }

  // Test 3: Local JSON playlist DB CRUD
  log('Test 3: Testing local database CRUD operations...');
  try {
    const testPlaylistName = '__crumusix_diagnostics_test_playlist__';
    const testPlaylist = { name: testPlaylistName, tracks: [] };

    // Save
    await invoke('save_playlist', { playlist: testPlaylist });
    log('  Playlist saved successfully.', 'success');

    // Get
    const playlists = await invoke('get_playlists');
    const found = playlists.some(p => p.name === testPlaylistName);
    if (found) {
      log('  Playlist retrieved successfully.', 'success');
    } else {
      throw new Error('Saved playlist not found in database retrieval.');
    }

    // Delete
    await invoke('delete_playlist', { name: testPlaylistName });
    log('  Playlist deleted successfully.', 'success');

    log('SUCCESS: Database CRUD functions are 100% operational.', 'success');
  } catch (err) {
    log(`ERROR: Database CRUD failed: ${err}`, 'error');
  }

  // Test 4: Spotify Premium Authentication status
  log('Test 4: Checking Spotify auth status...');
  if (appState.spotifyToken) {
    log('  Spotify Auth Session: Connected', 'success');
    const isExpired = Date.now() > appState.spotifyTokenExpiry * 1000;
    log(`  Token Status: ${isExpired ? 'EXPIRED' : 'ACTIVE'}`, isExpired ? 'warning' : 'success');
    if (isExpired) {
      log('  Attempting auto token refresh...');
      await refreshSpotifyAccessToken();
      const stillExpired = Date.now() > appState.spotifyTokenExpiry * 1000;
      log(`  Refresh Status: ${stillExpired ? 'FAILED' : 'SUCCESS'}`, stillExpired ? 'error' : 'success');
    }
  } else {
    log('  Spotify Auth Session: Disconnected (Connect in Spotify Auth tab for full streaming)', 'warning');
  }

  // Test 5: Playback players
  log('Test 5: Checking playback drivers...');
  log(`  YouTube Player API: ${ytPlayer && ytPlayer.loadVideoById ? 'LOADED & READY' : 'NOT INITIALIZED'}`, ytPlayer && ytPlayer.loadVideoById ? 'success' : 'warning');
  log(`  Spotify Native Engine: ${appState.spotifyDeviceId ? 'CONNECTED & ACTIVE' : 'DISCONNECTED'}`, appState.spotifyDeviceId ? 'success' : 'warning');
  log('Diagnostics complete!', 'warning');
}

// ==========================================================================
// 10. Native Queue Coordinator & Sidebar Renderer
// ==========================================================================
async function initNativeQueueState() {
  try {
    const queue = await invoke('queue_get_state');
    console.log('Initial native queue state retrieved:', queue);

    // Clean any corrupted URLs in the loaded queue items
    if (queue && queue.items) {
      queue.items.forEach(item => {
        if (item.artwork_url && (item.artwork_url.includes('file:') || item.artwork_url.includes('asset:') || item.artwork_url.includes('crumusix_cache'))) {
          item.artwork_url = '';
        }
      });
    }

    appState.queue = queue.items;
    appState.queueIndex = queue.current_index;
    appState.isShuffle = queue.is_shuffle;
    appState.repeatMode = queue.repeat_mode;

    document.getElementById('ctrl-shuffle').classList.toggle('active', appState.isShuffle);

    const repeatBtn = document.getElementById('ctrl-repeat');
    if (appState.repeatMode === 'track') {
      repeatBtn.classList.add('active');
      repeatBtn.setAttribute('title', 'Repeat: Track');
    } else if (appState.repeatMode === 'queue') {
      repeatBtn.classList.add('active');
      repeatBtn.setAttribute('title', 'Repeat: Queue');
    } else {
      repeatBtn.classList.remove('active');
      repeatBtn.setAttribute('title', 'Repeat: Off');
    }

    renderQueueSidebar();
  } catch (err) {
    console.error('Failed to initialize native queue state:', err);
  }
}

async function playPlaylistNative(tracks) {
  try {
    await invoke('queue_clear');
    for (const track of tracks) {
      await invoke('queue_add_track', {
        item: {
          track_id: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album || "",
          duration_ms: getTrackDurationMs(track),
          artwork_url: track.thumbnail || "",
          source: track.source
        }
      });
    }
    await invoke('queue_set_current_index', { index: 0 });
    playTrackImmediate(tracks[0]);
  } catch (err) {
    console.error('Failed to play playlist natively:', err);
  }
}

async function playTrackFromPlaylistNative(tracks, idx) {
  try {
    await invoke('queue_clear');
    for (const track of tracks) {
      await invoke('queue_add_track', {
        item: {
          track_id: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album || "",
          duration_ms: getTrackDurationMs(track),
          artwork_url: track.thumbnail || "",
          source: track.source
        }
      });
    }
    await invoke('queue_set_current_index', { index: idx });
    playTrackImmediate(tracks[idx]);
  } catch (err) {
    console.error('Failed to play track from playlist natively:', err);
  }
}

function renderQueueSidebar() {
  const container = document.getElementById('queue-list-container');
  if (!container) return;
  container.innerHTML = '';

  if (appState.queue.length === 0) {
    container.innerHTML = `
      <div style="padding: 24px; text-align: center; color: var(--color-text-dim); font-size: 0.9rem;">
        Queue is empty. Add some tracks to get started!
      </div>
    `;
    return;
  }

  function createQueueRowCard(track, actualIndex, isActive = false, isDraggable = false) {
    let artSrc = 'assets/default_art.png';
    if (track.artwork_url) {
      if (track.artwork_url.startsWith('/') || track.artwork_url.startsWith('C:\\') || track.artwork_url.includes('com.zech.crumusix') || track.artwork_url.includes('crumusix_cache')) {
        artSrc = getTauriAssetUrl(track.artwork_url);
      } else {
        artSrc = track.artwork_url;
      }
    }

    const row = document.createElement('div');
    row.className = `queue-row ${isActive ? 'active' : ''}`;
    row.innerHTML = `
      <img src="${artSrc}" class="queue-row-art" />
      <div class="queue-row-details">
        <div class="queue-row-title">${track.title}</div>
        <div class="queue-row-artist">${track.artist}</div>
      </div>
      <button class="queue-remove-btn" title="Remove track">&times;</button>
    `;

    row.querySelector('.queue-remove-btn').addEventListener('click', (e) => {
      e.stopPropagation();
      invoke('queue_remove_track', { index: actualIndex }).catch(console.error);
    });

    row.addEventListener('click', () => {
      invoke('queue_set_current_index', { index: actualIndex })
        .then(() => playTrackImmediate({
          id: track.track_id,
          title: track.title,
          artist: track.artist,
          album: track.album,
          duration: formatSeconds(track.duration_ms / 1000),
          source: track.source,
          thumbnail: track.artwork_url
        }))
        .catch(console.error);
    });

    // Setup HTML5 Drag & Drop handlers if enabled
    if (isDraggable) {
      row.setAttribute('draggable', 'true');

      row.addEventListener('dragstart', (e) => {
        e.dataTransfer.setData('text/plain', actualIndex);
        row.classList.add('dragging');
      });

      row.addEventListener('dragend', () => {
        row.classList.remove('dragging');
      });

      row.addEventListener('dragover', (e) => {
        e.preventDefault();
      });

      row.addEventListener('drop', (e) => {
        e.preventDefault();
        const fromIdx = parseInt(e.dataTransfer.getData('text/plain'));
        const toIdx = actualIndex;

        if (!isNaN(fromIdx) && fromIdx !== toIdx) {
          invoke('queue_reorder', { fromIndex: fromIdx, toIndex: toIdx })
            .catch(console.error);
        }
      });
    }

    return row;
  }

  // 1. NOW PLAYING
  if (appState.queueIndex >= 0 && appState.queueIndex < appState.queue.length) {
    const header = document.createElement('div');
    header.className = 'queue-section-header';
    header.textContent = 'Now Playing';
    container.appendChild(header);

    const track = appState.queue[appState.queueIndex];
    container.appendChild(createQueueRowCard(track, appState.queueIndex, true, false));
  }

  // 2. UP NEXT
  const upcomingTracks = appState.queue.slice(appState.queueIndex + 1);
  if (upcomingTracks.length > 0) {
    const header = document.createElement('div');
    header.className = 'queue-section-header';
    header.textContent = 'Up Next';
    container.appendChild(header);

    upcomingTracks.forEach((track, offset) => {
      const actualIndex = appState.queueIndex + 1 + offset;
      container.appendChild(createQueueRowCard(track, actualIndex, false, true));
    });
  }

  // 3. HISTORY
  if (appState.queueIndex > 0) {
    const header = document.createElement('div');
    header.className = 'queue-section-header';
    header.textContent = 'History';
    container.appendChild(header);

    const historyTracks = appState.queue.slice(0, appState.queueIndex);
    historyTracks.forEach((track, actualIdx) => {
      container.appendChild(createQueueRowCard(track, actualIdx, false, false));
    });
  }
}

// ==========================================================================
// Right Panel Tab Switching Logic
// ==========================================================================
document.addEventListener('DOMContentLoaded', () => {
  const tabBtns = document.querySelectorAll('.panel-tab-btn');
  const panelContents = document.querySelectorAll('.right-panel-content');

  tabBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      tabBtns.forEach(b => b.classList.remove('active'));
      panelContents.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const targetPanel = btn.getAttribute('data-panel');
      const content = document.getElementById(targetPanel);
      if (content) content.classList.add('active');
    });
  });

  initCommandPalette();
});

// Update Right Side Lyrics & Info Panel inside updatePlayerUI
const originalUpdatePlayerUI = updatePlayerUI;
updatePlayerUI = function (track) {
  originalUpdatePlayerUI(track);

  // Update Info Panel
  const infoContainer = document.getElementById('track-info-container');
  if (infoContainer) {
    infoContainer.innerHTML = `
      <div class="info-item"><label>Title</label><span>${track.title}</span></div>
      <div class="info-item"><label>Artist</label><span>${track.artist}</span></div>
      <div class="info-item"><label>Album</label><span>${track.album || "Unknown Album"}</span></div>
      <div class="info-item"><label>Duration</label><span>${track.duration}</span></div>
      <div class="info-item"><label>Source Engine</label><span style="text-transform: capitalize; color: var(--color-accent); font-weight:700;">${track.source} Core</span></div>
      <div class="info-item"><label>Track Reference ID</label><span style="font-family:monospace; font-size:0.82rem;">${track.id}</span></div>
    `;
  }

  // Set default static lyrics placeholder
  const lyricsContainer = document.getElementById('lyrics-container');
  if (lyricsContainer) {
    lyricsContainer.innerHTML = `<div class="results-loading">Resolving lyrics from LRCLIB...</div>`;
  }

  appState.currentLyrics = null;

  // Fetch lyrics natively from LRCLIB cache/provider
  const durationMs = getTrackDurationMs(track);
  invoke('lyrics_get_for_track', {
    trackId: track.id,
    title: track.title,
    artist: track.artist,
    album: track.album || "",
    durationMs: durationMs
  })
    .then(lyrics => {
      appState.currentLyrics = lyrics;
      renderTrackLyrics(lyrics);
    })
    .catch(err => {
      console.warn("Failed to fetch lyrics:", err);
      renderTrackLyricsMissing(track, err);
    });
};

function renderTrackLyrics(lyrics) {
  const container = document.getElementById('lyrics-container');
  if (!container) return;

  container.innerHTML = "";

  if (lyrics.lines.length === 0) {
    container.innerHTML = `<div class="lyrics-placeholder">No lyrics available for this track.</div>`;
    return;
  }

  const scrollBox = document.createElement('div');
  scrollBox.className = "lyrics-scroll-box";
  scrollBox.style.cssText = "display: flex; flex-direction: column; gap: 20px; text-align: center; width: 100%; padding-bottom: 50%;";

  lyrics.lines.forEach((line, idx) => {
    const lineEl = document.createElement('div');
    lineEl.className = "lyric-line";
    lineEl.style.cssText = "font-size: 1.15rem; font-weight: 500; opacity: 0.45; transition: all 0.3s ease; line-height: 1.6; user-select: text; cursor: pointer;";
    lineEl.textContent = line.text || "•••";

    if (lyrics.synced && line.timestamp_ms !== null) {
      lineEl.setAttribute('data-timestamp', line.timestamp_ms);

      // Allow clicking a lyric line to seek directly to it!
      lineEl.addEventListener('click', () => {
        const pct = (line.timestamp_ms / getTrackDurationMs(appState.currentTrack)) * 100;
        seekPlayback(pct);
      });
    }

    scrollBox.appendChild(lineEl);
  });

  container.appendChild(scrollBox);
}

function renderTrackLyricsMissing(track, err) {
  const container = document.getElementById('lyrics-container');
  if (!container) return;

  container.innerHTML = `
    <div class="lyrics-placeholder" style="margin-top: 60px;">
      <svg viewBox="0 0 24 24" width="48" height="48" stroke="var(--color-text-dim)" stroke-width="1.5" fill="none" style="margin-bottom:16px;"><path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>
      <p style="font-weight: 600; color: var(--color-text-muted); margin-bottom: 8px;">No lyrics available</p>
      <p style="font-size: 0.85rem; color: var(--color-text-dim); max-width: 250px; margin: 0 auto 16px;">We couldn't resolve plain or synced lyrics from LRCLIB for this track.</p>
      <div style="display: flex; gap: 8px; justify-content: center;">
        <button class="action-btn" id="btn-refresh-lyrics" style="padding: 6px 12px; font-size:0.85rem;">Retry Fetch</button>
        <button class="action-btn" id="btn-manual-lyrics-search" style="padding: 6px 12px; font-size:0.85rem; background: var(--color-accent); color: white; border-color: var(--color-accent);">Search Manually</button>
      </div>
    </div>
  `;

  document.getElementById('btn-refresh-lyrics').addEventListener('click', () => {
    updatePlayerUI(track);
  });

  const manualSearchBtn = document.getElementById('btn-manual-lyrics-search');
  if (manualSearchBtn) {
    manualSearchBtn.addEventListener('click', () => {
      openManualLyricsModal(track);
    });
  }
}

function syncActiveLyricLine(positionMs) {
  const scrollBox = document.querySelector('.lyrics-scroll-box');
  if (!scrollBox) return;

  const lines = scrollBox.querySelectorAll('.lyric-line');
  let activeLine = null;
  let activeIdx = -1;

  // Apply manual Timing Offset calibration
  const adjustedPosition = positionMs + appState.lyricOffsetMs;

  for (let i = 0; i < lines.length; i++) {
    const ts = parseInt(lines[i].getAttribute('data-timestamp'));
    if (!isNaN(ts) && ts <= adjustedPosition) {
      activeLine = lines[i];
      activeIdx = i;
    }
  }

  if (activeLine && !activeLine.classList.contains('active')) {
    lines.forEach(l => {
      l.classList.remove('active');
      l.style.opacity = "0.45";
      l.style.transform = "scale(1)";
      l.style.color = "var(--color-text-main)";
    });

    activeLine.classList.add('active');
    activeLine.style.opacity = "1";
    activeLine.style.transform = "scale(1.08)";
    activeLine.style.color = "var(--color-accent)";

    activeLine.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
}

// ==========================================================================
// Global Command Palette Logic (Ctrl+K Launcher)
// ==========================================================================
function initCommandPalette() {
  const palette = document.getElementById('command-palette');
  const input = document.getElementById('palette-search-input');
  const resultsContainer = document.getElementById('palette-results-list');

  if (!palette || !input || !resultsContainer) return;

  const commands = [
    { title: "Navigate to Listen Now", action: () => switchScreen('home'), type: "Navigation" },
    { title: "Navigate to Search Catalog", action: () => switchScreen('search'), type: "Navigation" },
    { title: "Navigate to Library Playlists", action: () => switchScreen('playlists'), type: "Navigation" },
    { title: "Navigate to Spotify Connection", action: () => switchScreen('spotify-connect'), type: "Navigation" },
    { title: "Navigate to Settings", action: () => switchScreen('settings'), type: "Navigation" },
    { title: "Toggle Play / Pause", action: () => togglePlayPause(), type: "Playback" },
    { title: "Skip to Next Track", action: () => playNext(), type: "Playback" },
    { title: "Go to Previous Track", action: () => playPrev(), type: "Playback" },
    { title: "Toggle Play Queue Sidebar", action: () => document.getElementById('btn-toggle-queue').click(), type: "Layout" },
    { title: "Clear Audio Play Queue", action: () => invoke('queue_clear').catch(console.error), type: "Queue" },
    { title: "Trigger Diagnostics Test", action: () => document.getElementById('btn-run-diagnostics').click(), type: "System" }
  ];

  let selectedIdx = 0;
  let activeItems = [];

  function showPalette() {
    palette.classList.remove('palette-hidden');
    input.value = "";
    input.focus();
    renderPaletteResults(commands);
  }

  function hidePalette() {
    palette.classList.add('palette-hidden');
    input.blur();
  }

  function renderPaletteResults(items) {
    resultsContainer.innerHTML = "";
    activeItems = items;
    selectedIdx = 0;

    if (items.length === 0) {
      resultsContainer.innerHTML = `<div style="padding: 16px; color: var(--color-text-dim); text-align:center;">No commands or tracks found.</div>`;
      return;
    }

    items.forEach((item, idx) => {
      const row = document.createElement('div');
      row.className = `palette-item ${idx === selectedIdx ? 'selected' : ''}`;
      row.innerHTML = `
        <span>${item.title}</span>
        <span class="palette-item-action">${item.type}</span>
      `;

      row.addEventListener('click', () => {
        item.action();
        hidePalette();
      });

      resultsContainer.appendChild(row);
    });
  }

  // Keyboard navigation & search trigger listeners
  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      showPalette();
    }

    if (e.key === 'Escape' && !palette.classList.contains('palette-hidden')) {
      hidePalette();
    }
  });

  input.addEventListener('input', () => {
    const rawVal = input.value;
    const query = rawVal.toLowerCase().trim();
    if (!query) {
      renderPaletteResults(commands);
      return;
    }

    // 1. Dynamic Search Catalog Shortcut (s: query)
    if (query.startsWith('s:')) {
      const searchTerms = rawVal.substring(2).trim();
      if (searchTerms) {
        renderPaletteResults([{
          title: `Search Catalog for: "${searchTerms}"`,
          action: () => {
            switchScreen('search');
            const searchInput = document.getElementById('search-input');
            if (searchInput) {
              searchInput.value = searchTerms;
              const searchClearBtn = document.getElementById('search-clear-btn');
              if (searchClearBtn) searchClearBtn.style.display = 'block';
            }
            performSearch(searchTerms);
          },
          type: "Catalog Search"
        }]);
        return;
      }
    }

    // 2. Dynamic Volume Shortcut (vol 0-100)
    if (query.startsWith('vol ')) {
      const volStr = query.substring(4).trim();
      const volVal = parseInt(volStr);
      if (!isNaN(volVal) && volVal >= 0 && volVal <= 100) {
        renderPaletteResults([{
          title: `Set system playback volume to ${volVal}%`,
          action: () => {
            appState.volume = volVal;
            saveConfig();
            const slider = document.getElementById('player-volume-slider');
            if (slider) slider.value = volVal;
            updateVolumeFill(volVal);
            setVolume(volVal);
          },
          type: "Volume Adjust"
        }]);
        return;
      }
    }

    const filtered = commands.filter(c => c.title.toLowerCase().includes(query) || c.type.toLowerCase().includes(query));
    renderPaletteResults(filtered);
  });

  input.addEventListener('keydown', (e) => {
    const items = resultsContainer.querySelectorAll('.palette-item');

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (items.length === 0) return;
      items[selectedIdx].classList.remove('selected');
      selectedIdx = (selectedIdx + 1) % items.length;
      items[selectedIdx].classList.add('selected');
      items[selectedIdx].scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (items.length === 0) return;
      items[selectedIdx].classList.remove('selected');
      selectedIdx = (selectedIdx - 1 + items.length) % items.length;
      items[selectedIdx].classList.add('selected');
      items[selectedIdx].scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (activeItems[selectedIdx]) {
        activeItems[selectedIdx].action();
        hidePalette();
      }
    }
  });

  // Close palette on backdrop click
  palette.addEventListener('click', (e) => {
    if (e.target === palette) {
      hidePalette();
    }
  });
}

// ==========================================================================
// Always-On-Top Mini Player Mode Trigger
// ==========================================================================
async function toggleMiniPlayerMode() {
  appState.isMiniPlayer = !appState.isMiniPlayer;

  try {
    await invoke('toggle_mini_player', { isMini: appState.isMiniPlayer });
    document.body.classList.toggle('mini-player-active', appState.isMiniPlayer);
    console.log(`Mini player mode toggled: ${appState.isMiniPlayer}`);
  } catch (err) {
    console.error("Failed to toggle native mini player:", err);
  }
}

// ==========================================================================
// Active Audio Device & Desktop OS Notifications
// ==========================================================================
async function updateAudioOutputDevice() {
  try {
    const deviceName = await invoke('get_audio_output_device');
    const displayEl = document.getElementById('settings-audio-device');
    if (displayEl) {
      displayEl.textContent = deviceName;
    }
    console.log("CPAL Active output device:", deviceName);
  } catch (err) {
    console.error("Failed to detect audio output device:", err);
  }
}

// Request desktop notification permission on startup
if (window.Notification && Notification.permission !== 'granted') {
  Notification.requestPermission();
}

function showNativeNotification(track) {
  if (!window.Notification || Notification.permission !== 'granted') return;

  let artSrc = 'assets/default_art.png';
  if (track.thumbnail && !track.thumbnail.startsWith('data:')) {
    artSrc = track.thumbnail;
  }

  try {
    new Notification("Now Playing — CrumusiX", {
      body: `${track.title}\nArtist: ${track.artist}`,
      icon: artSrc,
      silent: true // Do not play system overlay sounds
    });
  } catch (err) {
    console.error("Failed to trigger desktop notification:", err);
  }
}

// Hook desktop notification trigger into updatePlayerUI
const originalPlayerUIForNotify = updatePlayerUI;
updatePlayerUI = function (track) {
  originalPlayerUIForNotify(track);
  showNativeNotification(track);
};

// Keyboard listener for media control and view shortcuts
window.addEventListener('keydown', (e) => {
  // If the user is typing in a text field, do not trigger global shortcuts (except modifier shortcuts)
  const activeTag = document.activeElement ? document.activeElement.tagName.toLowerCase() : '';
  const isInputActive = activeTag === 'input' || activeTag === 'textarea' || (document.activeElement && document.activeElement.isContentEditable);

  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'm') {
    e.preventDefault();
    toggleMiniPlayerMode();
    return;
  }

  // Ctrl+Shift+S: Jump to search and focus input
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 's') {
    e.preventDefault();
    switchScreen('search');
    const searchInput = document.getElementById('search-input');
    if (searchInput) searchInput.focus();
    return;
  }

  // Ctrl+Shift+L: Toggle play queue sidebar
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'l') {
    e.preventDefault();
    const toggleQueueBtn = document.getElementById('btn-toggle-queue');
    if (toggleQueueBtn) toggleQueueBtn.click();
    return;
  }

  // Escape: Close modals, clear active search, or collapse queue sidebar
  if (e.key === 'Escape') {
    e.preventDefault();
    const searchInput = document.getElementById('search-input');
    if (searchInput && document.activeElement === searchInput) {
      searchInput.value = '';
      const clearBtn = document.getElementById('search-clear-btn');
      if (clearBtn) clearBtn.style.display = 'none';
      clearSearchResults();
      searchInput.blur();
    }
    const queueSidebar = document.querySelector('.app-queue-sidebar');
    if (queueSidebar && !queueSidebar.classList.contains('queue-sidebar-hidden')) {
      const toggleQueueBtn = document.getElementById('btn-toggle-queue');
      if (toggleQueueBtn) toggleQueueBtn.click();
    }
    return;
  }

  // Global media shortcuts (only active when not typing inside an input)
  if (!isInputActive) {
    // Space: Play/Pause
    if (e.key === ' ') {
      e.preventDefault();
      togglePlayPause();
    }
    // ArrowUp: Volume +5%
    else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const newVol = Math.min(100, appState.volume + 5);
      appState.volume = newVol;
      saveConfig();
      const slider = document.getElementById('player-volume-slider');
      if (slider) slider.value = newVol;
      updateVolumeFill(newVol);
      setVolume(newVol);
    }
    // ArrowDown: Volume -5%
    else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const newVol = Math.max(0, appState.volume - 5);
      appState.volume = newVol;
      saveConfig();
      const slider = document.getElementById('player-volume-slider');
      if (slider) slider.value = newVol;
      updateVolumeFill(newVol);
      setVolume(newVol);
    }
    // ArrowLeft: Seek -5 seconds (mocking seek using current percentage/estimated playback)
    else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      const seekSlider = document.getElementById('player-seek-slider');
      if (seekSlider) {
        let currentPct = parseFloat(seekSlider.value || '0');
        let newPct = Math.max(0, currentPct - 2); // 2% seek jump
        seekSlider.value = newPct;
        document.getElementById('player-seek-fill').style.width = newPct + '%';
        seekPlayback(newPct);
      }
    }
    // ArrowRight: Seek +5 seconds
    else if (e.key === 'ArrowRight') {
      e.preventDefault();
      const seekSlider = document.getElementById('player-seek-slider');
      if (seekSlider) {
        let currentPct = parseFloat(seekSlider.value || '0');
        let newPct = Math.min(100, currentPct + 2); // 2% seek jump
        seekSlider.value = newPct;
        document.getElementById('player-seek-fill').style.width = newPct + '%';
        seekPlayback(newPct);
      }
    }
  }
});


// Bind click on btn-toggle-mini-video or custom action if needed
document.addEventListener('DOMContentLoaded', () => {
  const winMiniplayerBtn = document.getElementById('win-miniplayer');
  if (winMiniplayerBtn) {
    winMiniplayerBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      e.preventDefault();
      toggleMiniPlayerMode();
    });
  }

  const miniVideoBtn = document.getElementById('btn-toggle-mini-video');
  if (miniVideoBtn) {
    // Modify to double as a mini player trigger
    miniVideoBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      e.preventDefault();
      toggleMiniPlayerMode();
    });
    miniVideoBtn.setAttribute('title', 'Toggle Always-On-Top Mini Player (Ctrl+Shift+M)');
  }
  // Initialize dynamic timing calibration and fullscreen lyrics buttons
  initLyricsControls();

  // Wire up Purge Lyrics Cache settings button
  const purgeBtn = document.getElementById('btn-purge-lyrics-cache');
  if (purgeBtn) {
    purgeBtn.addEventListener('click', async () => {
      const confirmPurge = await confirm("Are you sure you want to purge all cached offline lyrics? This action cannot be undone.");
      if (confirmPurge) {
        try {
          await invoke('lyrics_purge_cache');
          await alert("Offline lyrics cache successfully cleared.");
          updateLyricsCacheStats();
        } catch (err) {
          console.error("Failed to purge lyrics cache:", err);
          await alert(`Error purging cache: ${err}`);
        }
      }
    });
  }

  // Detect audio device on boot
  updateAudioOutputDevice();
});

async function updateLyricsCacheStats() {
  const countEl = document.getElementById('settings-lyrics-cache-count');
  if (!countEl) return;
  try {
    const stats = await invoke('lyrics_get_cache_stats');
    countEl.textContent = stats;
  } catch (err) {
    console.error("Failed to fetch lyrics cache count:", err);
    countEl.textContent = "Error";
  }
}

// ==========================================================================
// Cinematic Ambient Backdrop & Queue Prefetching Integrations
// ==========================================================================
function updateLyricsBackdropColors(imgUrl, trackId, artist) {
  const blob1 = document.querySelector('.glow-blob-1');
  const blob2 = document.querySelector('.glow-blob-2');
  const blob3 = document.querySelector('.glow-blob-3');

  if (!blob1 || !blob2 || !blob3) return;

  // Extremely beautiful and robust HSL generator fallback
  const getHashColor = (str, s = 80, l = 40) => {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const h = Math.abs(hash) % 360;
    return `hsl(${h}, ${s}%, ${l}%)`;
  };

  const color1 = getHashColor(trackId || 'default1', 85, 30);
  const color2 = getHashColor(artist || 'default2', 90, 40);
  const color3 = getHashColor((trackId + artist) || 'default3', 75, 35);

  // Set default hash colors dynamically (guarantees stunning gradients instantly)
  blob1.style.background = color1;
  blob2.style.background = color2;
  blob3.style.background = color3;

  // Attempt to extract dynamic vibrant colors from the artwork covers if CORS allows
  if (!imgUrl || imgUrl.startsWith('data:')) return;

  const img = new Image();
  img.crossOrigin = "anonymous";
  img.src = imgUrl;
  img.onload = () => {
    try {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      canvas.width = 30;
      canvas.height = 30;
      ctx.drawImage(img, 0, 0, 30, 30);
      const imgData = ctx.getImageData(0, 0, 30, 30).data;

      const getAvgColor = (startX, startY, endX, endY) => {
        let r = 0, g = 0, b = 0, count = 0;
        for (let y = startY; y < endY; y++) {
          for (let x = startX; x < endX; x++) {
            const idx = (y * 30 + x) * 4;
            r += imgData[idx];
            g += imgData[idx + 1];
            b += imgData[idx + 2];
            count++;
          }
        }
        return `rgb(${Math.round(r / count)}, ${Math.round(g / count)}, ${Math.round(b / count)})`;
      };

      blob1.style.background = getAvgColor(0, 0, 15, 15);
      blob2.style.background = getAvgColor(15, 15, 30, 30);
      blob3.style.background = getAvgColor(0, 15, 15, 30);
    } catch (e) {
      console.warn("Dynamic image color extraction restricted by CORS; falling back to premium HSL hash colors.", e);
    }
  };
}

function prefetchUpcomingTrackLyrics() {
  if (!appState.queue || appState.queue.length === 0) return;
  const nextIdx = appState.queueIndex + 1;
  if (nextIdx < appState.queue.length) {
    const upcomingTrack = appState.queue[nextIdx];
    const durationMs = getTrackDurationMs(upcomingTrack);

    console.log(`[Prefetch] Triggering background prefetch/caching for: ${upcomingTrack.title}`);
    invoke('lyrics_get_for_track', {
      trackId: upcomingTrack.id,
      title: upcomingTrack.title,
      artist: upcomingTrack.artist,
      album: upcomingTrack.album || "",
      durationMs: durationMs
    }).then(() => {
      console.log(`[Prefetch] Successfully cached lyrics in background for: ${upcomingTrack.title}`);
    }).catch(err => {
      console.warn(`[Prefetch] Backend lookup missed for ${upcomingTrack.title}:`, err);
    });
  }
}

function initLyricsControls() {
  const minusBtn = document.getElementById('btn-lyric-sync-minus');
  const plusBtn = document.getElementById('btn-lyric-sync-plus');
  const offsetLabel = document.getElementById('lyric-sync-offset-label');
  const fullscreenBtn = document.getElementById('btn-lyric-fullscreen');
  const panelLyrics = document.getElementById('panel-lyrics');

  if (minusBtn && plusBtn && offsetLabel) {
    minusBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      appState.lyricOffsetMs -= 500;
      offsetLabel.textContent = `${(appState.lyricOffsetMs / 1000).toFixed(1)}s`;
      console.log(`Manual lyrics sync offset: ${appState.lyricOffsetMs}ms`);
    });

    plusBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      appState.lyricOffsetMs += 500;
      offsetLabel.textContent = `${(appState.lyricOffsetMs / 1000).toFixed(1)}s`;
      console.log(`Manual lyrics sync offset: ${appState.lyricOffsetMs}ms`);
    });
  }

  if (fullscreenBtn && panelLyrics) {
    fullscreenBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      panelLyrics.classList.toggle('lyrics-fullscreen-active');
      const isFullscreen = panelLyrics.classList.contains('lyrics-fullscreen-active');
      fullscreenBtn.style.color = isFullscreen ? 'var(--color-accent)' : '';
      fullscreenBtn.setAttribute('title', isFullscreen ? 'Exit Full-Screen' : 'Toggle Full-Screen Lyrics');
      console.log(`Full-screen lyrics layout active: ${isFullscreen}`);
    });
  }
}

function openManualLyricsModal(track) {
  const modal = document.getElementById('manual-lyrics-modal');
  const artistInput = document.getElementById('manual-lyrics-artist-input');
  const titleInput = document.getElementById('manual-lyrics-title-input');
  const resultsList = document.getElementById('manual-lyrics-results-list');
  const closeBtn = document.getElementById('btn-close-manual-lyrics');
  const triggerBtn = document.getElementById('btn-manual-lyrics-search-trigger');

  if (!modal || !artistInput || !titleInput || !resultsList) return;

  // Show modal
  modal.classList.remove('palette-hidden');

  // Pre-fill inputs with current track info
  artistInput.value = track.artist || "";
  titleInput.value = track.title || "";

  resultsList.innerHTML = `<div class="results-loading" style="text-align: center; color: var(--color-text-dim); padding: 10px;">Verify terms and click Search...</div>`;

  const closeModal = () => {
    modal.classList.add('palette-hidden');
  };

  closeBtn.onclick = closeModal;

  // Modal dismiss on click outside
  modal.onclick = (e) => {
    if (e.target === modal) {
      closeModal();
    }
  };

  triggerBtn.onclick = () => {
    const artistVal = artistInput.value.trim();
    const titleVal = titleInput.value.trim();

    if (!artistVal || !titleVal) {
      resultsList.innerHTML = `<div style="color: #ef4444; text-align: center; padding: 10px; font-size: 0.88rem;">Please fill in both fields.</div>`;
      return;
    }

    resultsList.innerHTML = `<div class="results-loading" style="text-align: center; color: var(--color-text-dim); padding: 15px;">Searching LRCLIB database...</div>`;

    const durationMs = getTrackDurationMs(track);

    // Call native command with original track.id but user-defined search terms
    invoke('lyrics_get_for_track', {
      trackId: track.id,
      title: titleVal,
      artist: artistVal,
      album: track.album || "",
      durationMs: durationMs
    })
      .then(lyrics => {
        resultsList.innerHTML = `
        <div style="padding: 15px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px;">
          <p style="color: var(--color-accent); font-weight: 700; margin: 0; font-size: 0.95rem;">Lyrics Found!</p>
          <p style="font-size: 0.82rem; color: var(--color-text-dim); margin: 0;">Successfully parsed ${lyrics.synced ? "Synced (LRC)" : "Plain"} lyrics.</p>
          <button class="primary-btn" id="btn-apply-manual-lyrics" style="padding: 8px 14px; font-size: 0.85rem; width: auto; margin-top: 6px;">Apply & Cache Permanently</button>
        </div>
      `;

        const applyBtn = document.getElementById('btn-apply-manual-lyrics');
        if (applyBtn) {
          applyBtn.onclick = () => {
            appState.currentLyrics = lyrics;
            renderTrackLyrics(lyrics);
            closeModal();
          };
        }
      });
  };
}

// ==========================================================================
// Local Offline Library Scanning, Search & Playback Integration
// ==========================================================================
function initLocalLibrary() {
  const btnScan = document.getElementById('btn-library-scan');
  const pathInput = document.getElementById('library-scan-path');
  const scanStatus = document.getElementById('library-scan-status');
  const searchInput = document.getElementById('library-search-input');

  if (btnScan) {
    btnScan.addEventListener('click', async () => {
      const dirPath = pathInput.value.trim();
      if (!dirPath) {
        await alert("Please enter a valid absolute path to a music folder.");
        return;
      }

      btnScan.disabled = true;
      scanStatus.textContent = "Scanning directory recursively... This might take a moment.";
      scanStatus.style.color = "var(--color-accent)";

      try {
        const count = await invoke('library_scan_dir_async', { dirPath });
        scanStatus.textContent = `Success! Scanned and indexed ${count} offline tracks.`;
        scanStatus.style.color = "#1DB954";
        loadLibraryCatalog();
      } catch (err) {
        scanStatus.textContent = `Error: ${err}`;
        scanStatus.style.color = "#ef4444";
      } finally {
        btnScan.disabled = false;
      }
    });
  }

  if (searchInput) {
    searchInput.addEventListener('input', async (e) => {
      const query = e.target.value.trim();
      const container = document.getElementById('library-results-list');

      if (!query) {
        loadLibraryCatalog();
        return;
      }

      container.innerHTML = '<div class="results-loading">Searching local catalog...</div>';
      try {
        const tracks = await invoke('library_search', { query });
        renderLibraryTracks(tracks);
      } catch (err) {
        console.error("Local search failed:", err);
        container.innerHTML = `<div class="results-loading" style="color:#ef4444;">Search error: ${err}</div>`;
      }
    });
  }
}

async function loadLibraryCatalog() {
  const container = document.getElementById('library-results-list');
  if (!container) return;

  container.innerHTML = '<div class="results-loading">Loading local catalog...</div>';
  try {
    const tracks = await invoke('library_get_all');
    renderLibraryTracks(tracks);
  } catch (err) {
    console.error("Failed to load catalog:", err);
    container.innerHTML = `<div class="results-loading" style="color:#ef4444;">Failed to load catalog: ${err}</div>`;
  }
}

function renderLibraryTracks(tracks) {
  const container = document.getElementById('library-results-list');
  if (!container) return;

  container.innerHTML = '';
  if (tracks.length === 0) {
    container.innerHTML = '<div class="search-initial">No local tracks found. Try scanning a folder or clear search filter.</div>';
    return;
  }

  tracks.forEach((track, idx) => {
    const row = createTrackRowHTML(track, idx + 1);
    container.appendChild(row);
  });
}

function startDiagnosticsPolling() {
  setInterval(async () => {
    if (appState.activeScreen === 'settings') {
      try {
        const diag = await invoke('get_diagnostics_snapshot');

        const ramEl = document.getElementById('diag-ram');
        if (ramEl) ramEl.textContent = `${diag.ram_usage_mb.toFixed(1)} MB`;

        const latencyEl = document.getElementById('diag-latency');
        if (latencyEl) latencyEl.textContent = `${diag.provider_latency_ms} ms`;

        const bufferEl = document.getElementById('diag-buffer');
        if (bufferEl) bufferEl.textContent = `${diag.buffer_health_pct}%`;

        const cacheEl = document.getElementById('diag-cache');
        if (cacheEl) {
          const total = diag.cache_hits + diag.cache_misses;
          const ratio = total > 0 ? ((diag.cache_hits / total) * 100).toFixed(0) : 100;
          cacheEl.textContent = `${ratio}% (${diag.cache_hits}/${total})`;
        }
      } catch (err) {
        console.error("Failed to query diagnostics:", err);
      }
    }
  }, 1500);
}
