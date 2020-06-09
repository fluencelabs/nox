# Fluence deployment scripts
`deployment_config.json` contains list of IPs to use for deployment
`fab deploy_fluence` – deploys fluence, mediated by `fluence.yml` and `fluence_bootstrap.yml`
`fab install_docker` – installs docker and docker-compose (+ haveged)
`fab deploy_watchdog` – deploys a watchdog to monitor containers (change `SECRET` to desired webhook URL)
`fab deploy_caddy` – deploys Caddy 2.0, configured in code

# Prometheus
`/prometheus` contains basic configuration file, HTML consoles are TBD
