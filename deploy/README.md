# How to deploy Fluence
1. Edit deployment_config.json to your needs (explanations: TBD)
2. Install docker: `fab install_docker`
3. Edit `fluence.yml` and `fluence_bootstrap.yml` to your needs
4. Deploy fluence: `fab deploy_fluence`
5. If you need https, deploy caddy: `fab deploy_caddy`
6. If you need slack notifications about containers state, deploy watchdog: `fab deploy_watchdog`

# Fluence deployment scripts and configs
`deployment_config.json` – contains list of IPs to use for deployment
`fab deploy_fluence` – deploys fluence, mediated by `fluence.yml` and `fluence_bootstrap.yml`
`fab install_docker` – installs docker and docker-compose (+ haveged)
`fab deploy_watchdog` – deploys a watchdog to monitor containers (change `SECRET` to desired webhook URL)
`fab deploy_caddy` – deploys Caddy 2.0, configured in code

# Prometheus
`/prometheus` contains basic configuration file, HTML consoles are TBD

