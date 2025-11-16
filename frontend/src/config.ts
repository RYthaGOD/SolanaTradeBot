// API Configuration
// This will use the environment variable in production (Vercel)
// and fallback to localhost for development

export const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export default {
  apiUrl: API_BASE_URL,
};
