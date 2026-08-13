import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Locale = "en" | "zh";

const STORAGE = "proofship.locale";

type I18nCtx = {
  locale: Locale;
  setLocale: (l: Locale) => void;
};

const Ctx = createContext<I18nCtx>({ locale: "en", setLocale: () => {} });

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>("en");

  useEffect(() => {
    try {
      const v = window.localStorage.getItem(STORAGE);
      if (v === "zh" || v === "en") setLocaleState(v);
    } catch {
      /* ignore */
    }
  }, []);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    try {
      window.localStorage.setItem(STORAGE, l);
    } catch {
      /* ignore */
    }
  }, []);
  const value = useMemo(() => ({ locale, setLocale }), [locale, setLocale]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useLocale() {
  return useContext(Ctx);
}

export function pick<T>(locale: Locale, en: T, zh: T): T {
  return locale === "zh" ? zh : en;
}
