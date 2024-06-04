# Example Rust-based SMART-on-FHIR app

This is a Rust-based SMART-on-FHIR application that recreates [Cerner's SMART-on-FHIR tutorial](https://engineering.cerner.com/smart-on-fhir-tutorial/#standalone-app-launch-for-patient-access-workflow)
using [Actix](https://actix.rs) and [Maud](https://maud.lambda.xyz).

## License

This repository is licensed Apache 2.0, with several exceptions:

* This repository packages in several Javascript libraries, under `lib`
* Two CSS and JS resource files are copied in verbatim from [Cerner's tutorial repo](https://github.com/cerner/smart-on-fhir-tutorial/tree/gh-pages),
  these files are under `resources`

## Architecture

The current application is a straightforward translation of Cerner's SMART-on-FHIR tutorial from hardcoded HTML into
HTML macros that are defined using [Maud](https://maud.lambda.xyz) and served using [Actix](https://actix.rs).
We then package the app into a [Docker](https://docker.com) container, which is deployed via
[AWS Fargate](https://aws.amazon.com/fargate/).

### Application architecture

Our application exposes three primary endpoints:

* `/`, defined in `src/index.rs`
* `/launch.html`, defined in `src/launch.rs`
* `/healthcheck.html`, defined in `src/health.rs`

It also exposes endpoints to serve the contents of the `/lib` and `/resources` directories,
using the [actix_files](https://docs.rs/actix-files/latest/actix_files/) crate.

The `/healthcheck.html` endpoint provides a simple mechanism to check if the server is running.

The `/launch.html` endpoint is the endpoint that a FHIR application would call to launch your
SMART-on-FHIR application. This endpoint is responsible for starting the SMART authorization
sequence, and requesting the necessary [Oauth scopes](https://build.fhir.org/ig/HL7/smart-app-launch/scopes-and-launch-context.html)
for your application.

The `/` endpoint is the endpoint that a FHIR application would redirect to, after launching your
application. At this point, your application will have the necessary credentials to access data
using FHIR.

At present, we are using the (included) `fhir-client` javascript libraries to start the SMART
authorization sequence, and to request data using FHIR. Over the next few commits, we will migrate
to a fully Rust-based implementation.

### Deployment architecture

The app is packaged into a simple Docker container, using the `Dockerfile` in the root directory.
Once the container is built, it is pushed to [AWS ECR](https://aws.amazon.com/ecr/);
example instructions can be found [here](https://docs.aws.amazon.com/AmazonECR/latest/userguide/getting-started-cli.html).

We manage our deployment using [Terraform](http://terraform.io/docs/providers/aws/r/route53_record.html),
which can be found in the `iac` directory. We make a few assumptions:

* You are deploying this application under a subdomain (e.g., `fhir-example.translating.science`, vs
  `translating.science`) of a root domain
* You have a multiple Account organization structure in AWS, where your DNS for your root domain
  is managed in an `Infrastructure` account that is separate from the account where you plan to
  deploy the `fhir-example` application (an example of this can be found [here](https://aws.amazon.com/blogs/mt/scale-multi-account-architecture-aws-network-firewall-and-aws-control-tower/))

We have two deployment steps:

* `iac/dns`, which is used initially to set up DNS records and certificates. This allows us to
  deploy the app behind HTTPS in Fargate, with AWS managing the certificats via [AWS ACM](https://aws.amazon.com/certificate-manager/).
* `iac/fargate`, which sets up Fargate and its accompanying resources.

We use several Terraform variables:

* `aws_profile`: Used by both `dns` and `fargate`. This is the name of the AWS profile used to
  deploy into the AWS account for the `fhir-example` application.
* `aws_profile_main`: Used only by `dns`. This is the name of the AWS profile used to deploy
  into the AWS `Infrastructure` account.
* `domain`: Used only by `dns`. This is the name of the root domain.
* `subdomain`: Used by both `dns` and `fargate`. This is the name of the subdomain.
* `image`: Used only by `fargate`. This is the repository/tag of the Docker image to use.

There are additional optional variables defined in the IaC.

### Deploying

We have an example deployment running at [https://fhir-example.translating.science](https://fhir-example.translating.science).
If you'd like to run your own deployment, you can follow the steps below.

#### Building a Docker container

Assuming you have [Docker installed](https://docs.docker.com/engine/install/), you can
build a Docker container by running `docker build .` in the repository root.

#### Pushing the Docker container to ECR

Once you have built your Docker container, you should create a private repository in the
AWS account you plan to deploy the application in. There are numerous ways to do this,
which are documented on the [AWS site](https://docs.aws.amazon.com/AmazonECR/latest/userguide/repository-create.html). Right now, we do not do this in Terraform, but we could.

Once you have created your repository, you can push your container to ECR by running
Step 4 and onwards in [this AWS tutorial](https://docs.aws.amazon.com/AmazonECR/latest/userguide/getting-started-cli.html).

#### Deploying DNS records and validating certificates

Move into the `iac/dns` directory, and run `terraform init` to initialize the terraform
providers.

To create the [terraform plan](https://developer.hashicorp.com/terraform/tutorials/cli/plan) for this step, run:

```
terraform plan  -var="domain=<mydomain>" -var="subdomain=fhir-example.<mydomain>" -var="aws_profile=<application_profile>" -var="aws_profile_main=<infrastructure_profile>" -out=plan
```

This is a good time to check the generated plan to make sure it looks correct. Assuming
that your plan creates successfully, you can then create the resources by running:

```
terraform apply plan
```

This step should take approximately 5 minutes.

#### Deploying the application

Move into the `iac/fargate` directory, and run `terraform init`. To generate a plan, you
will need to run:

```
terraform plan -var="subdomain=fhir-example.<mydomain>" -var="aws_profile=<application_profile>" -var="image=<application_account>.dkr.ecr.us-west-2.amazonaws.com/<repository_name>:<tag>" -out=plan
```

Check your plan, and if it looks correct, deploy by running `terraform apply plan`.
This step takes approximately 1-3 minutes to run. Once the resources are created, it takes
several minutes for Fargate to launch your instances and start your container.

#### Launch your SMART-on-FHIR application

You can launch your application by going to the [SMART Sandbox Launcher](https://launch.smarthealthit.org/). You will want to use the `R2 (DSTU2)` FHIR version, and the `Provider EHR` launch type.
