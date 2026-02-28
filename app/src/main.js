import {listen} from '@tauri-apps/api/event';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {LogicalSize} from '@tauri-apps/api/dpi';

const appWindow = getCurrentWindow();

const wordEl = document.getElementById('word');
const phoneticEl = document.getElementById('phonetic');
const translationEl = document.getElementById('translation');
const definitionsEl = document.getElementById('definitions');
const examplesEl = document.getElementById('examples');
const audioBtn = document.getElementById('audio-btn');
const audioPlayer = document.getElementById('audio-player');

let currentAudioUrl = '';

// Listen for translation results from Rust backend
listen('translation-result', (event) => {
    const data = event.payload;
    render(data);
    if (data.auto_play && data.audio_url) {
        playAudio(data.audio_url);
    }
});

function render(data) {
    wordEl.textContent = data.word;
    phoneticEl.textContent = data.phonetic ? `/${data.phonetic}/` : '';
    translationEl.textContent = data.translation;
    currentAudioUrl = data.audio_url || '';

    // Render definitions
    definitionsEl.innerHTML = '';
    if (data.definitions && data.definitions.length > 0) {
        const shown = data.definitions.slice(0, 6);
        for (const def of shown) {
            const item = document.createElement('div');
            item.className = 'def-item';
            item.innerHTML = `<span class="pos">${escapeHtml(def.part_of_speech)}</span>${escapeHtml(def.meaning)}`;
            definitionsEl.appendChild(item);
        }
    }

    // Render examples
    examplesEl.innerHTML = '';
    if (data.examples && data.examples.length > 0) {
        const shown = data.examples.slice(0, 2);
        for (const ex of shown) {
            const item = document.createElement('div');
            item.className = 'example-item';
            item.textContent = ex;
            examplesEl.appendChild(item);
        }
    }

    // Resize the window to fit the content after DOM updates
    requestAnimationFrame(() => {
        resizeToFit();
    });
}

function resizeToFit() {
    const container = document.getElementById('popup');
    // Size the window to exactly match the popup container (no extra padding)
    const width = container.offsetWidth;
    const height = container.offsetHeight;
    appWindow.setSize(new LogicalSize(width, height));
}

function playAudio(url) {
    audioPlayer.src = url;
    audioBtn.classList.add('playing');
    audioPlayer.play().catch(() => {
        audioBtn.classList.remove('playing');
    });
}

audioPlayer.addEventListener('ended', () => {
    audioBtn.classList.remove('playing');
});

audioPlayer.addEventListener('error', () => {
    audioBtn.classList.remove('playing');
});

audioBtn.addEventListener('click', () => {
    if (currentAudioUrl) {
        playAudio(currentAudioUrl);
    }
});

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
