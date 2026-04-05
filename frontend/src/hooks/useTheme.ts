import { useState, useEffect, useCallback } from 'react';
import { Theme } from '../types';

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(() => {
    return (localStorage.getItem('dab-theme') as Theme) || 'system';
  });

  const getEffectiveTheme = useCallback((): 'light' | 'dark' => {
    if (theme === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return theme;
  }, [theme]);

  const [effectiveTheme, setEffectiveTheme] = useState<'light' | 'dark'>(getEffectiveTheme);

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme);
    localStorage.setItem('dab-theme', newTheme);
  }, []);

  useEffect(() => {
    const effective = getEffectiveTheme();
    setEffectiveTheme(effective);

    if (effective === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme, getEffectiveTheme]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => {
      if (theme === 'system') {
        const effective = mediaQuery.matches ? 'dark' : 'light';
        setEffectiveTheme(effective);
        if (effective === 'dark') {
          document.documentElement.classList.add('dark');
        } else {
          document.documentElement.classList.remove('dark');
        }
      }
    };
    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, [theme]);

  return { theme, effectiveTheme, setTheme };
}
