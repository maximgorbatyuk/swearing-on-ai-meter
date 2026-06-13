// Typing animation
document.addEventListener('DOMContentLoaded', () => {
  const el = document.querySelector('[data-typing]');
  if (!el) return;

  const text = el.getAttribute('data-typing');
  let i = 0;

  // Respect reduced motion
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    el.textContent = text;
    return;
  }

  // Clear fallback text before animating
  el.textContent = '';

  function type() {
    if (i < text.length) {
      el.textContent += text.charAt(i);
      i++;
      setTimeout(type, 50);
    }
  }

  setTimeout(type, 400);
});

// Scroll-triggered fade-in
document.addEventListener('DOMContentLoaded', () => {
  if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.1 });

    document.querySelectorAll('.animate-on-scroll').forEach(el => {
      observer.observe(el);
    });
  } else {
    document.querySelectorAll('.animate-on-scroll').forEach(el => {
      el.classList.add('visible');
    });
  }
});

// Latest release activity
document.addEventListener('DOMContentLoaded', async () => {
  const output = document.getElementById('release-status-content');
  if (!output) return;

  const releasesUrl = 'https://github.com/maximgorbatyuk/swearing-on-ai-meter/releases';
  const apiUrl = 'https://api.github.com/repos/maximgorbatyuk/swearing-on-ai-meter/releases/latest';
  const cacheKey = 'soaim_latest_release_v1';
  const cacheTtlMs = 60 * 60 * 1000; // 1 hour

  const humanize = (publishedAtIso) => {
    const publishedAt = new Date(publishedAtIso);
    if (Number.isNaN(publishedAt.getTime())) return null;

    const diffMs = Date.now() - publishedAt.getTime();
    if (diffMs < 0) return 'just now';

    const minute = 60 * 1000;
    const hour = 60 * minute;
    const day = 24 * hour;
    const month = 30 * day;
    const year = 365 * day;

    if (diffMs < hour) {
      const n = Math.max(1, Math.floor(diffMs / minute));
      return `${n} minute${n === 1 ? '' : 's'} ago`;
    }
    if (diffMs < day) {
      const n = Math.floor(diffMs / hour);
      return `${n} hour${n === 1 ? '' : 's'} ago`;
    }
    if (diffMs < month) {
      const n = Math.floor(diffMs / day);
      return `${n} day${n === 1 ? '' : 's'} ago`;
    }
    if (diffMs < year) {
      const n = Math.floor(diffMs / month);
      return `${n} month${n === 1 ? '' : 's'} ago`;
    }
    const n = Math.floor(diffMs / year);
    return `${n} year${n === 1 ? '' : 's'} ago`;
  };

  const formatExactDate = (publishedAtIso) => {
    const publishedAt = new Date(publishedAtIso);
    if (Number.isNaN(publishedAt.getTime())) return null;
    return new Intl.DateTimeFormat('en-US', {
      year: 'numeric',
      month: 'short',
      day: '2-digit'
    }).format(publishedAt);
  };

  const sanitizeReleaseUrl = (rawUrl) => {
    try {
      const parsed = new URL(rawUrl);
      if (parsed.protocol !== 'https:') return releasesUrl;
      if (parsed.hostname !== 'github.com' && parsed.hostname !== 'www.github.com') {
        return releasesUrl;
      }
      return parsed.toString();
    } catch (_) {
      return releasesUrl;
    }
  };

  const clearOutput = () => {
    output.textContent = '';
  };

  const appendLine = (text) => {
    output.appendChild(document.createTextNode(text));
    output.appendChild(document.createTextNode('\n'));
  };

  const renderUnavailable = () => {
    clearOutput();
    appendLine('No releases published yet.');
    output.appendChild(document.createTextNode('Watch for releases: '));
    const link = document.createElement('a');
    link.href = releasesUrl;
    link.target = '_blank';
    link.rel = 'noopener';
    link.textContent = 'github.com/maximgorbatyuk/swearing-on-ai-meter/releases';
    output.appendChild(link);
  };

  const renderRelease = ({ tag, publishedAt, releaseUrl, fromCache = false }) => {
    const relative = humanize(publishedAt);
    const exactDate = formatExactDate(publishedAt);
    if (!relative || !exactDate) {
      renderUnavailable();
      return;
    }

    clearOutput();
    appendLine(`Latest release: ${tag}`);
    appendLine(`Published: ${exactDate} (${relative})`);
    output.appendChild(document.createTextNode('Release notes: '));
    const link = document.createElement('a');
    link.href = sanitizeReleaseUrl(releaseUrl);
    link.target = '_blank';
    link.rel = 'noopener';
    link.textContent = 'open';
    output.appendChild(link);

    if (fromCache) {
      output.appendChild(document.createTextNode('\nSource: cached GitHub response'));
    }
  };

  const readCache = () => {
    try {
      const raw = localStorage.getItem(cacheKey);
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== 'object') return null;
      if (typeof parsed.cachedAt !== 'number') return null;
      if (!parsed.release || typeof parsed.release !== 'object') return null;
      return parsed;
    } catch (_) {
      return null;
    }
  };

  const writeCache = (release) => {
    try {
      localStorage.setItem(
        cacheKey,
        JSON.stringify({
          cachedAt: Date.now(),
          release
        })
      );
    } catch (_) {
      // Ignore storage failures (private mode, quota limits, etc.)
    }
  };

  const cached = readCache();
  if (cached && Date.now() - cached.cachedAt < cacheTtlMs) {
    renderRelease({ ...cached.release, fromCache: true });
    return;
  }

  try {
    const response = await fetch(apiUrl, {
      headers: { Accept: 'application/vnd.github+json' }
    });
    if (!response.ok) throw new Error(`GitHub API error: ${response.status}`);

    const release = await response.json();
    const normalized = {
      tag: typeof release.tag_name === 'string' && release.tag_name.trim()
        ? release.tag_name.trim()
        : 'unknown',
      publishedAt: release.published_at,
      releaseUrl: typeof release.html_url === 'string' ? release.html_url : releasesUrl
    };

    if (!normalized.publishedAt) {
      throw new Error('Missing release date');
    }

    writeCache(normalized);
    renderRelease(normalized);
  } catch (_) {
    if (cached?.release) {
      renderRelease({ ...cached.release, fromCache: true });
      return;
    }
    renderUnavailable();
  }
});
