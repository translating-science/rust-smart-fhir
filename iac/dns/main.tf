variable "domain" {
  type    = string
  default = null
}

variable "subdomain" {
  type    = string
  default = null
}

data "aws_route53_zone" "primary" {
  provider = aws.main
  name     = var.domain
}

resource "aws_route53_zone" "fhir_example" {
  name = var.subdomain
}

resource "aws_acm_certificate" "default" {
  domain_name       = var.subdomain
  validation_method = "DNS"
  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_route53_record" "subdomain_ns" {
  provider = aws.main

  allow_overwrite = true
  name            = var.subdomain

  records = [
    aws_route53_zone.fhir_example.name_servers[0],
    aws_route53_zone.fhir_example.name_servers[1],
    aws_route53_zone.fhir_example.name_servers[2],
    aws_route53_zone.fhir_example.name_servers[3],
  ]
  ttl     = 60
  type    = "NS"
  zone_id = data.aws_route53_zone.primary.zone_id
}

resource "aws_route53_record" "validation" {
  for_each = {
    for dvo in aws_acm_certificate.default.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.fhir_example.zone_id
}

resource "aws_route53_record" "primary_validation" {
  provider = aws.main

  for_each = {
    for dvo in aws_acm_certificate.default.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = data.aws_route53_zone.primary.zone_id
}


resource "aws_acm_certificate_validation" "default" {
  certificate_arn         = aws_acm_certificate.default.arn
  validation_record_fqdns = [for record in aws_route53_record.validation : record.fqdn]
}
