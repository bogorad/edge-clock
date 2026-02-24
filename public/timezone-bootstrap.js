(() => {
  const QUERY_PARAM_NAME = "tz";
  const COOKIE_NAME = "clock_tz";

  if (typeof window === "undefined") {
    return;
  }

  if (typeof Intl === "undefined" || typeof Intl.DateTimeFormat !== "function") {
    return;
  }

  const resolved = Intl.DateTimeFormat().resolvedOptions();
  const browserTimeZone = typeof resolved.timeZone === "string" ? resolved.timeZone : "";
  if (browserTimeZone.length === 0) {
    return;
  }

  const url = new URL(window.location.href);
  const queryTimeZone = url.searchParams.get(QUERY_PARAM_NAME);
  const cookieTimeZone = readCookie(COOKIE_NAME);

  if (queryTimeZone === browserTimeZone && cookieTimeZone === browserTimeZone) {
    url.searchParams.delete(QUERY_PARAM_NAME);
    const cleanUrl = `${url.pathname}${url.search}${url.hash}`;
    const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;

    if (cleanUrl !== currentUrl) {
      window.history.replaceState(null, "", cleanUrl);
    }

    return;
  }

  if (queryTimeZone !== browserTimeZone && cookieTimeZone !== browserTimeZone) {
    url.searchParams.set(QUERY_PARAM_NAME, browserTimeZone);
    window.location.replace(url.toString());
  }

  function readCookie(cookieName) {
    const cookies = document.cookie ? document.cookie.split(";") : [];

    for (const cookieEntry of cookies) {
      const separatorIndex = cookieEntry.indexOf("=");
      if (separatorIndex === -1) {
        continue;
      }

      const rawName = cookieEntry.slice(0, separatorIndex).trim();
      if (rawName !== cookieName) {
        continue;
      }

      const rawValue = cookieEntry.slice(separatorIndex + 1);
      try {
        return decodeURIComponent(rawValue);
      } catch {
        return null;
      }
    }

    return null;
  }
})();
