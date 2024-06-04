provider "aws" {
  region  = "us-west-2"
  profile = var.aws_profile
}

variable "aws_profile" {
  type    = string
  default = null
}

provider "aws" {
  alias   = "acm"
  region  = "us-east-1"
  profile = var.aws_profile
}

variable "aws_profile_main" {
  type    = string
  default = null
}

provider "aws" {
  alias   = "main"
  region  = "us-west-2"
  profile = var.aws_profile_main
}
