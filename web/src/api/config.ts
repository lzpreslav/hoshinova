import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Config } from '../bindings/Config';
import { rejectError } from './api';

export const useQueryConfig = () =>
  useQuery({
    queryKey: ['config'],
    queryFn: () =>
      fetch('/api/config')
        .then(rejectError)
        .then((res) => res.json())
        .then((res) => res as Config),
  });

export const useMutateReloadConfig = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      fetch('/api/config/reload', { method: 'POST' })
        .then(rejectError)
        .then((res) => res.json()),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['config'] });
      queryClient.invalidateQueries({ queryKey: ['config', 'toml'] });
    },
  });
};

export const useQueryConfigTOML = () =>
  useQuery({
    queryKey: ['config', 'toml'],
    queryFn: () =>
      fetch('/api/config/toml')
        .then(rejectError)
        .then((res) => res.text()),
  });

export const useMutateConfigTOML = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (toml: string) =>
      fetch('/api/config/toml', {
        method: 'PUT',
        body: toml,
        headers: { 'Content-Type': 'text/toml' },
      })
        .then(rejectError)
        .then((res) => res.text()),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['config'] });
      queryClient.invalidateQueries({ queryKey: ['config', 'toml'] });
    },
  });
};
