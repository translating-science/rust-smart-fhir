provider "aws" {
  region  = "us-west-2"
  profile = var.aws_profile
}

variable "aws_profile" {
  type    = string
  default = null
}

