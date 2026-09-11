# Plan — deferred / future ring
1. Stage-1 local (done): store, memo, engine, build, cli, agent surface.
2. Stage-2 opt-in (frozen C-SELF-05): wire bridge (net_fd), none = default.
3. Deep-memo nitro (F-403): probe + fallback; real shim future.
4. Stage-2 sync CoreLink (F-405): wire bridge crate ready; future.
5. Cross-tenant dedup: staged (post-GA).
6. Full networking (DNS/VPN): future.
7. Resource limits: needs ns/vz runtime; future.
8. Registry push: future.
9. O(1) view backends: spike (composefs/NFS/projfs); unwired.
10. Network registry + L2 switch (F-404): future ring; honest unsupported until spike green.
11. Mesh design (LAN cache) independent of C-SELF-05; design ARCHITECTURE.md §5.
12. Mesh F-404 deferred — LAN mesh cache (peer store discovery); design docs/ARCHITECTURE.md §5 (net_fd: socketpair AF_UNIX SOCK_DGRAM, lines 126/149/345); independent C-SELF-07; future ring; zero regression selfhosted base (net_fd: None).
